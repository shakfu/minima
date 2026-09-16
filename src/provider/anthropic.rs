//! anthropic-messages. Content blocks rather than flat messages, and the only dialect whose
//! parser needs the SSE `event:` name -- the data alone does not say what a frame is.
//!
//! Tool results are blocks on a *user* message, not a role of their own, so consecutive results
//! coalesce into one message.

use anyhow::anyhow;
use serde_json::{Value, json};

use super::{Error, Event, Message, Role, Usage};
use crate::config::Config;

/// Required by the wire, with no Chat equivalent. Generous: the cap that matters is the context
/// check in `agent.rs`.
const MAX_TOKENS: u32 = 8192;

pub fn build_body(cfg: &Config, messages: &[Message], tools: &[Value]) -> Value {
    let mut system = String::new();
    let mut wire: Vec<Value> = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                if let Some(text) = &m.content {
                    system.push_str(text);
                }
            }
            Role::User => wire.push(json!({
                "role": "user",
                "content": [{ "type": "text", "text": m.content.clone().unwrap_or_default() }],
            })),
            Role::Assistant => {
                let mut blocks = Vec::new();
                if let Some(text) = m.content.as_deref().filter(|t| !t.is_empty()) {
                    blocks.push(json!({ "type": "text", "text": text }));
                }
                for call in &m.tool_calls {
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.name,
                        // The wire wants a parsed object; a fragmentary or malformed argument
                        // string would otherwise fail the whole request.
                        "input": serde_json::from_str::<Value>(&call.arguments)
                            .unwrap_or_else(|_| json!({})),
                    }));
                }
                if !blocks.is_empty() {
                    wire.push(json!({ "role": "assistant", "content": blocks }));
                }
            }
            Role::Tool => {
                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                    "content": m.content.clone().unwrap_or_default(),
                });
                // One user message holds every result for an assistant turn, as Anthropic documents.
                // The API also combines consecutive user turns, so a prompt after a cancel is valid.
                match wire.last_mut() {
                    Some(last) if last["role"] == "user" => {
                        if let Some(content) = last["content"].as_array_mut() {
                            content.push(block);
                        }
                    }
                    _ => wire.push(json!({ "role": "user", "content": [block] })),
                }
            }
        }
    }

    let mut body = json!({
        "model": cfg.model,
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "messages": wire,
    });
    if !system.is_empty() {
        body["system"] = json!([{ "type": "text", "text": system }]);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }
    body
}

/// Flat, and the schema key is `input_schema`, not `parameters`.
pub fn tool_spec(name: &str, description: &str, parameters: Value) -> Value {
    json!({ "name": name, "description": description, "input_schema": parameters })
}

pub fn parse_frame(event: &str, data: &str) -> Vec<Result<Event, Error>> {
    let root: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => {
            return vec![Err(Error::Other(
                anyhow!(e).context("parsing a messages stream frame"),
            ))];
        }
    };

    match event {
        "message_start" => {
            let usage = &root["message"]["usage"];
            usage["input_tokens"]
                .as_u64()
                .map(|input| {
                    vec![Ok(Event::Usage(Usage::from_parts(
                        input as u32,
                        usage["output_tokens"].as_u64().unwrap_or(0) as u32,
                    )))]
                })
                .unwrap_or_default()
        }

        "content_block_start" => {
            let block = &root["content_block"];
            if block["type"].as_str() != Some("tool_use") {
                return Vec::new();
            }
            vec![Ok(Event::ToolCallDelta {
                key: root["index"].as_u64().unwrap_or(0).to_string(),
                id: block["id"].as_str().map(str::to_string),
                name: block["name"].as_str().map(str::to_string),
                arguments: None,
            })]
        }

        "content_block_delta" => {
            let delta = &root["delta"];
            let key = root["index"].as_u64().unwrap_or(0).to_string();
            match delta["type"].as_str() {
                Some("text_delta") => delta["text"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .map(|t| vec![Ok(Event::Text(t.to_string()))])
                    .unwrap_or_default(),
                Some("input_json_delta") => vec![Ok(Event::ToolCallDelta {
                    key,
                    id: None,
                    name: None,
                    arguments: delta["partial_json"].as_str().map(str::to_string),
                })],
                // thinking_delta and signature_delta belong to extended thinking, which minima does
                // not request; see TODO.md.
                _ => Vec::new(),
            }
        }

        // The final output count arrives here, but the input count only ever appeared in
        // message_start, so this is a correction rather than a whole figure.
        "message_delta" => {
            let mut out: Vec<_> = root["usage"]["output_tokens"]
                .as_u64()
                .map(|output| Ok(Event::Usage(Usage::from_parts(0, output as u32))))
                .into_iter()
                .collect();
            if root["delta"]["stop_reason"] == "max_tokens" {
                out.push(Ok(Event::Truncated));
            }
            out
        }

        "message_stop" => vec![Ok(Event::Done)],

        "error" => {
            let message = root["error"]["message"]
                .as_str()
                .unwrap_or("the request failed");
            vec![Err(Error::Other(anyhow!("{message}")))]
        }

        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ToolCall;

    #[test]
    fn the_system_prompt_is_top_level_not_a_message() {
        let cfg = Config::for_test("m");
        let body = build_body(
            &cfg,
            &[Message::system("be terse"), Message::user("hi")],
            &[],
        );
        assert_eq!(body["system"][0]["text"], "be terse");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["max_tokens"], MAX_TOKENS);
    }

    #[test]
    fn tool_calls_become_blocks_with_parsed_input() {
        let cfg = Config::for_test("m");
        let call = ToolCall {
            id: "t1".into(),
            name: "read".into(),
            arguments: r#"{"path":"x"}"#.into(),
        };
        let body = build_body(&cfg, &[Message::assistant(None, vec![call])], &[]);
        let block = &body["messages"][0]["content"][0];
        assert_eq!(block["type"], "tool_use");
        assert_eq!(block["input"]["path"], "x");
    }

    /// Two consecutive user messages are rejected by the wire, and every tool result is one.
    #[test]
    fn consecutive_tool_results_coalesce_into_one_user_message() {
        let cfg = Config::for_test("m");
        let body = build_body(
            &cfg,
            &[
                Message::tool_result("a", "one"),
                Message::tool_result("b", "two"),
            ],
            &[],
        );
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn malformed_tool_arguments_do_not_poison_the_request() {
        let cfg = Config::for_test("m");
        let call = ToolCall {
            id: "t1".into(),
            name: "read".into(),
            arguments: "{not json".into(),
        };
        let body = build_body(&cfg, &[Message::assistant(None, vec![call])], &[]);
        assert_eq!(body["messages"][0]["content"][0]["input"], json!({}));
    }

    #[test]
    fn the_event_name_selects_the_frame_meaning() {
        let text = parse_frame(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
        );
        assert_eq!(text[0].as_ref().unwrap(), &Event::Text("hi".into()));

        let args = parse_frame(
            "content_block_delta",
            r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"a\""}}"#,
        );
        assert_eq!(
            args[0].as_ref().unwrap(),
            &Event::ToolCallDelta {
                key: "1".into(),
                id: None,
                name: None,
                arguments: Some("{\"a\"".into()),
            }
        );

        assert_eq!(
            parse_frame("message_stop", "{}")[0].as_ref().unwrap(),
            &Event::Done
        );
        assert!(parse_frame("ping", "{}").is_empty());
    }

    #[test]
    fn a_max_tokens_stop_is_reported_as_truncation() {
        let events = parse_frame(
            "message_delta",
            r#"{"delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":8192}}"#,
        );
        assert_eq!(
            events[0].as_ref().unwrap(),
            &Event::Usage(Usage::from_parts(0, 8192))
        );
        assert_eq!(events[1].as_ref().unwrap(), &Event::Truncated);
    }
}
