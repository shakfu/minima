//! openai-responses. Typed input items rather than flat messages, and the system prompt is
//! `instructions` rather than a message.
//!
//! Tool arguments stream against the output item's `id`, while the result must quote `call_id`.
//! The two differ, so the opening `response.output_item.added` carries the mapping.

use anyhow::anyhow;
use serde_json::{Value, json};

use super::{Error, Event, Message, Role, Usage};
use crate::config::Config;

pub fn build_body(cfg: &Config, messages: &[Message], tools: &[Value], cache_key: &str) -> Value {
    let mut instructions = String::new();
    let mut input = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                if let Some(text) = &m.content {
                    instructions.push_str(text);
                }
            }
            Role::User => input.push(content_item("user", "input_text", m.content.as_deref())),
            Role::Assistant => {
                if let Some(text) = m.content.as_deref().filter(|t| !t.is_empty()) {
                    input.push(content_item("assistant", "output_text", Some(text)));
                }
                for call in &m.tool_calls {
                    input.push(json!({
                        "type": "function_call",
                        "call_id": call.id,
                        "name": call.name,
                        "arguments": call.arguments,
                    }));
                }
            }
            Role::Tool => input.push(json!({
                "type": "function_call_output",
                "call_id": m.tool_call_id.clone().unwrap_or_default(),
                "output": m.content.clone().unwrap_or_default(),
            })),
        }
    }

    let mut body = json!({
        "model": cfg.model,
        "stream": true,
        // No server-side conversation state: minima resends the transcript, and storing it would be
        // persistence the scope does not want.
        "store": false,
        "instructions": instructions,
        "input": input,
        "prompt_cache_key": cache_key,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("auto");
    }
    body
}

fn content_item(role: &str, kind: &str, text: Option<&str>) -> Value {
    json!({
        "type": "message",
        "role": role,
        "content": [{ "type": kind, "text": text.unwrap_or_default() }],
    })
}

/// Flat, unlike Chat: no `function` nesting.
pub fn tool_spec(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "name": name,
        "description": description,
        "parameters": parameters,
    })
}

pub fn parse_frame(data: &str) -> Vec<Result<Event, Error>> {
    let root: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => {
            return vec![Err(Error::Other(
                anyhow!(e).context("parsing a responses stream frame"),
            ))];
        }
    };
    let kind = root["type"].as_str().unwrap_or_default();

    match kind {
        "response.output_text.delta" | "response.refusal.delta" => root["delta"]
            .as_str()
            .filter(|t| !t.is_empty())
            .map(|t| vec![Ok(Event::Text(t.to_string()))])
            .unwrap_or_default(),

        "response.output_item.added" => {
            let item = &root["item"];
            if item["type"].as_str() != Some("function_call") {
                return Vec::new();
            }
            vec![Ok(Event::ToolCallDelta {
                key: item["id"].as_str().unwrap_or_default().to_string(),
                id: item["call_id"].as_str().map(str::to_string),
                name: item["name"].as_str().map(str::to_string),
                arguments: None,
            })]
        }

        "response.function_call_arguments.delta" => vec![Ok(Event::ToolCallDelta {
            key: root["item_id"].as_str().unwrap_or_default().to_string(),
            id: None,
            name: None,
            arguments: root["delta"].as_str().map(str::to_string),
        })],

        "response.completed" | "response.done" => {
            let usage = &root["response"]["usage"];
            let mut out = Vec::new();
            if let (Some(input), Some(output)) = (
                usage["input_tokens"].as_u64(),
                usage["output_tokens"].as_u64(),
            ) {
                // Cached tokens are a part of `input_tokens`, not an addition to it.
                let cached = usage["input_tokens_details"]["cached_tokens"].as_u64();
                out.push(Ok(Event::Usage(Usage {
                    cache_read: cached.unwrap_or(0) as u32,
                    ..Usage::from_parts(input as u32, output as u32)
                })));
            }
            out.push(Ok(Event::Done));
            out
        }

        "response.incomplete"
            if root["response"]["incomplete_details"]["reason"] == "max_output_tokens" =>
        {
            vec![Ok(Event::Truncated), Ok(Event::Done)]
        }

        "response.failed" | "response.incomplete" | "error" => {
            let message = root["response"]["error"]["message"]
                .as_str()
                .or_else(|| root["message"].as_str())
                .or_else(|| root["response"]["incomplete_details"]["reason"].as_str())
                .unwrap_or("the response failed");
            vec![Err(Error::Other(anyhow!("{message}")))]
        }

        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_prompt_becomes_instructions_not_a_message() {
        let cfg = Config::for_test("m");
        let body = build_body(
            &cfg,
            &[Message::system("be terse"), Message::user("hi")],
            &[],
            "k",
        );
        assert_eq!(body["instructions"], "be terse");
        assert_eq!(body["input"].as_array().unwrap().len(), 1);
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["store"], false);
    }

    #[test]
    fn a_tool_result_quotes_call_id_not_the_item_id() {
        let cfg = Config::for_test("m");
        let body = build_body(&cfg, &[Message::tool_result("call_7", "42")], &[], "k");
        assert_eq!(body["input"][0]["type"], "function_call_output");
        assert_eq!(body["input"][0]["call_id"], "call_7");
    }

    /// The added item supplies call_id and name; later fragments only carry item_id, so the two
    /// must group under the same key.
    #[test]
    fn argument_fragments_group_with_the_item_that_opened_them() {
        let opened = parse_frame(
            r#"{"type":"response.output_item.added",
                "item":{"type":"function_call","id":"item_1","call_id":"call_1","name":"read"}}"#,
        );
        let delta = parse_frame(
            r#"{"type":"response.function_call_arguments.delta",
                "item_id":"item_1","delta":"{\"p\":1}"}"#,
        );
        let key_of = |e: &Event| match e {
            Event::ToolCallDelta { key, .. } => key.clone(),
            other => panic!("expected a tool fragment, got {other:?}"),
        };
        assert_eq!(
            key_of(opened[0].as_ref().unwrap()),
            key_of(delta[0].as_ref().unwrap())
        );
    }

    #[test]
    fn completion_reports_usage_then_done() {
        let events = parse_frame(
            r#"{"type":"response.completed",
                "response":{"usage":{"input_tokens":10,"output_tokens":5}}}"#,
        );
        assert_eq!(
            events[0].as_ref().unwrap(),
            &Event::Usage(Usage::from_parts(10, 5))
        );
        assert_eq!(events[1].as_ref().unwrap(), &Event::Done);
    }

    #[test]
    fn cached_input_is_a_part_of_the_input() {
        let events = parse_frame(
            r#"{"type":"response.completed","response":{"usage":{"input_tokens":10,
                "input_tokens_details":{"cached_tokens":8},"output_tokens":5}}}"#,
        );
        let Ok(Event::Usage(usage)) = &events[0] else {
            panic!("{events:?}")
        };
        assert_eq!((usage.prompt_tokens, usage.cache_read), (10, 8));
    }

    #[test]
    fn incomplete_at_the_output_limit_is_truncation_and_otherwise_an_error() {
        let limit = parse_frame(
            r#"{"type":"response.incomplete",
                "response":{"incomplete_details":{"reason":"max_output_tokens"}}}"#,
        );
        assert_eq!(limit[0].as_ref().unwrap(), &Event::Truncated);
        assert_eq!(limit[1].as_ref().unwrap(), &Event::Done);

        let filtered = parse_frame(
            r#"{"type":"response.incomplete",
                "response":{"incomplete_details":{"reason":"content_filter"}}}"#,
        );
        let err = filtered[0].as_ref().unwrap_err().to_string();
        assert!(err.contains("content_filter"), "{err}");
    }

    #[test]
    fn unknown_frames_are_ignored_rather_than_failing() {
        assert!(parse_frame(r#"{"type":"response.in_progress"}"#).is_empty());
    }
}
