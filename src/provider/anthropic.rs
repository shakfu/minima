//! anthropic-messages. Content blocks rather than flat messages, and the only dialect whose
//! parser needs the SSE `event:` name -- the data alone does not say what a frame is.
//!
//! Tool results are blocks on a *user* message, not a role of their own, so consecutive results
//! coalesce into one message.

use anyhow::anyhow;
use serde_json::{Value, json};

use super::{Error, Event, Message, Role, Usage};
use crate::config::Config;

/// Required by the wire, with no Chat equivalent. Used when the model list gives no ceiling; every
/// current Claude model accepts it, and a `write` of a whole file needs the room.
const DEFAULT_MAX_TOKENS: u32 = 32_000;

/// `room` is what the window has left, and no response can be longer. Claude models before 4.5
/// reject a request whose input plus `max_tokens` exceeds the window; later ones stop at it.
pub fn build_body(cfg: &Config, messages: &[Message], tools: &[Value], room: u32) -> Value {
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
                // Thinking blocks open a response, before its text and calls.
                let mut blocks = m.replay.clone();
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

    mark_previous_request(&mut wire);
    let mut body = json!({
        "model": cfg.model,
        "max_tokens": cfg.max_output.unwrap_or(DEFAULT_MAX_TOKENS).min(room).max(1),
        "stream": true,
        "messages": wire,
        // Anthropic caches only up to a marked block. The top-level marker is automatic caching:
        // the API marks the last block, so each request reads the prefix the previous one wrote.
        "cache_control": { "type": "ephemeral" },
    });
    if !system.is_empty() {
        body["system"] = json!([{ "type": "text", "text": system }]);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }
    // Thinking is on by default on Claude 5 models and off on 4.6 to 4.8 until asked for. Adaptive
    // lets the model skip it on easy turns. minima does not show it, and omitting the text lets
    // the answer start streaming sooner; the blocks still carry what must be replayed.
    if cfg.adaptive_thinking {
        body["thinking"] = json!({ "type": "adaptive", "display": "omitted" });
    }
    body
}

/// Marks the end of what the previous request sent: the message before the last assistant turn.
/// A read looks back at most 20 blocks from a marker, so a turn of ten parallel tool calls would
/// otherwise put the previous request's cache entry out of reach of the automatic marker.
fn mark_previous_request(wire: &mut [Value]) {
    let Some(last_turn) = wire.iter().rposition(|m| m["role"] == "assistant") else {
        return;
    };
    let Some(previous) = last_turn.checked_sub(1) else {
        return;
    };
    let block = wire[previous]["content"]
        .as_array_mut()
        .and_then(|blocks| blocks.last_mut());
    // An empty text block cannot carry a marker, and the API rejects it.
    if let Some(block) = block.filter(|b| b["text"] != "") {
        block["cache_control"] = json!({ "type": "ephemeral" });
    }
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
        // `input_tokens` excludes the cached parts, so the prompt is the sum of all three.
        "message_start" => {
            let usage = &root["message"]["usage"];
            let count = |key: &str| usage[key].as_u64().unwrap_or(0) as u32;
            usage["input_tokens"]
                .as_u64()
                .map(|input| {
                    let (read, write) = (
                        count("cache_read_input_tokens"),
                        count("cache_creation_input_tokens"),
                    );
                    vec![Ok(Event::Usage(Usage {
                        cache_read: read,
                        cache_write: write,
                        ..Usage::from_parts(input as u32 + read + write, count("output_tokens"))
                    }))]
                })
                .unwrap_or_default()
        }

        "content_block_start" => {
            let block = &root["content_block"];
            let key = root["index"].as_u64().unwrap_or(0).to_string();
            match block["type"].as_str() {
                Some("tool_use") => vec![Ok(Event::ToolCallDelta {
                    key,
                    id: block["id"].as_str().map(str::to_string),
                    name: block["name"].as_str().map(str::to_string),
                    arguments: None,
                })],
                // A redacted block arrives whole; a thinking block fills in from its deltas.
                Some("thinking" | "redacted_thinking") => vec![Ok(Event::Replay {
                    key,
                    part: block.clone(),
                })],
                _ => Vec::new(),
            }
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
                Some("thinking_delta") => vec![Ok(Event::Replay {
                    key,
                    part: json!({ "thinking": delta["thinking"] }),
                })],
                Some("signature_delta") => vec![Ok(Event::Replay {
                    key,
                    part: json!({ "signature": delta["signature"] }),
                })],
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
            u32::MAX,
        );
        assert_eq!(body["system"][0]["text"], "be terse");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    /// The model's own ceiling wins over the default, and the window's remainder over both.
    #[test]
    fn max_tokens_is_the_models_ceiling_bounded_by_the_room_left() {
        let mut cfg = Config::for_test("m");
        cfg.max_output = Some(64_000);
        let ask = |room| build_body(&cfg, &[Message::user("hi")], &[], room)["max_tokens"].clone();
        assert_eq!(ask(u32::MAX), 64_000);
        assert_eq!(ask(10_000), 10_000);
    }

    /// The top-level marker caches the whole request; the explicit one keeps the previous
    /// request's entry within the 20-block lookback when a turn adds many blocks.
    #[test]
    fn the_cache_is_marked_at_the_end_and_where_the_previous_request_ended() {
        let cfg = Config::for_test("m");
        let call = |id: &str| ToolCall {
            id: id.into(),
            name: "read".into(),
            arguments: "{}".into(),
        };
        let body = build_body(
            &cfg,
            &[
                Message::system("be terse"),
                Message::user("hi"),
                Message::assistant(None, vec![call("a"), call("b")]),
                Message::tool_result("a", "one"),
                Message::tool_result("b", "two"),
            ],
            &[],
            u32::MAX,
        );
        assert_eq!(body["cache_control"], json!({ "type": "ephemeral" }));
        let marked: Vec<_> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|m| m["content"].as_array().unwrap())
            .filter(|b| b.get("cache_control").is_some())
            .collect();
        assert_eq!(marked.len(), 1, "{marked:?}");
        assert_eq!(marked[0]["text"], "hi");
    }

    /// The API rejects a marker on an empty text block.
    #[test]
    fn an_empty_prompt_is_not_marked() {
        let cfg = Config::for_test("m");
        let body = build_body(
            &cfg,
            &[
                Message::user(""),
                Message::assistant(Some("x".into()), vec![]),
            ],
            &[],
            u32::MAX,
        );
        assert!(
            body["messages"][0]["content"][0]
                .get("cache_control")
                .is_none()
        );
    }

    /// Replayed first and unchanged: the API drops thinking for the rest of a tool-use turn when
    /// the blocks are missing, and rejects them altered.
    #[test]
    fn thinking_blocks_are_parsed_and_replayed_ahead_of_the_call() {
        let mut assembler = crate::turn::Assembler::new();
        for (event, data) in [
            (
                "content_block_start",
                r#"{"index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"thinking_delta","thinking":"let me "}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"thinking_delta","thinking":"look"}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"signature_delta","signature":"SIG"}}"#,
            ),
            (
                "content_block_start",
                r#"{"index":1,"content_block":{"type":"redacted_thinking","data":"XYZ"}}"#,
            ),
            (
                "content_block_start",
                r#"{"index":2,"content_block":{"type":"tool_use","id":"t1","name":"read"}}"#,
            ),
            ("message_stop", "{}"),
        ] {
            for e in parse_frame(event, data) {
                assembler.push(e.unwrap());
            }
        }
        let turn = assembler.finish().unwrap();
        assert_eq!(
            turn.replay,
            [
                json!({"type": "thinking", "thinking": "let me look", "signature": "SIG"}),
                json!({"type": "redacted_thinking", "data": "XYZ"}),
            ]
        );

        let mut message = Message::assistant(None, turn.calls);
        message.replay = turn.replay;
        let body = build_body(&Config::for_test("m"), &[message], &[], u32::MAX);
        let kinds: Vec<_> = body["messages"][0]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["thinking", "redacted_thinking", "tool_use"]);
    }

    #[test]
    fn adaptive_thinking_is_requested_only_where_the_model_list_offers_it() {
        let mut cfg = Config::for_test("m");
        let body = |cfg: &Config| build_body(cfg, &[Message::user("hi")], &[], u32::MAX);
        assert!(body(&cfg).get("thinking").is_none());
        cfg.adaptive_thinking = true;
        assert_eq!(
            body(&cfg)["thinking"],
            json!({"type": "adaptive", "display": "omitted"})
        );
    }

    #[test]
    fn tool_calls_become_blocks_with_parsed_input() {
        let cfg = Config::for_test("m");
        let call = ToolCall {
            id: "t1".into(),
            name: "read".into(),
            arguments: r#"{"path":"x"}"#.into(),
        };
        let body = build_body(&cfg, &[Message::assistant(None, vec![call])], &[], u32::MAX);
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
            u32::MAX,
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
        let body = build_body(&cfg, &[Message::assistant(None, vec![call])], &[], u32::MAX);
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

    /// Anthropic counts cached input apart from `input_tokens`, so the prompt is the sum.
    #[test]
    fn cached_input_is_added_to_the_prompt() {
        let events = parse_frame(
            "message_start",
            r#"{"message":{"usage":{"input_tokens":3,"cache_read_input_tokens":90,
                "cache_creation_input_tokens":7,"output_tokens":1}}}"#,
        );
        let Ok(Event::Usage(usage)) = &events[0] else {
            panic!("{events:?}")
        };
        assert_eq!(
            (usage.prompt_tokens, usage.cache_read, usage.cache_write),
            (100, 90, 7)
        );
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
