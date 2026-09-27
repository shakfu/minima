//! openai-chat: Chat Completions. Flat messages, tool calls keyed by index.

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Error, Event, Message, Role, Usage};
use crate::config::Config;

fn role(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

pub fn build_body(cfg: &Config, messages: &[Message], tools: &[Value], cache_key: &str) -> Value {
    let wire: Vec<Value> = messages
        .iter()
        .map(|m| {
            let mut out = json!({ "role": role(m.role) });
            if let Some(content) = &m.content {
                out["content"] = json!(content);
            } else if m.role == Role::Assistant && m.tool_calls.is_empty() {
                // An assistant message needs content or tool calls, and an empty turn has neither.
                out["content"] = json!("");
            }
            if !m.tool_calls.is_empty() {
                out["tool_calls"] = Value::Array(
                    m.tool_calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": { "name": c.name, "arguments": c.arguments },
                            })
                        })
                        .collect(),
                );
            }
            if let Some(id) = &m.tool_call_id {
                out["tool_call_id"] = json!(id);
            }
            if !m.replay.is_empty() {
                out["reasoning_details"] = json!(m.replay);
            }
            out
        })
        .collect();

    let mut body = json!({
        "model": cfg.model,
        "messages": wire,
        "stream": true,
        "stream_options": { "include_usage": true },
        "prompt_cache_key": cache_key,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }
    // OpenAI and Gemini models cache on their own, and Claude only up to a marked block. This is
    // OpenRouter's automatic caching, which marks the last block; other routes are not sent it.
    if cfg.provider == "openrouter" && cfg.model.starts_with("anthropic/") {
        body["cache_control"] = json!({ "type": "ephemeral" });
    }
    body
}

/// `{"type":"function","function":{name, description, parameters}}`
pub fn tool_spec(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": { "name": name, "description": description, "parameters": parameters },
    })
}

pub fn parse_frame(data: &str) -> Vec<Result<Event, Error>> {
    if data.trim() == "[DONE]" {
        return vec![Ok(Event::Done)];
    }
    match serde_json::from_str::<Chunk>(data) {
        Ok(chunk) => chunk.into_events(),
        Err(e) => vec![Err(Error::Other(
            anyhow!(e).context("parsing a chat stream chunk"),
        ))],
    }
}

#[derive(Deserialize)]
struct Chunk {
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<WireUsage>,
    /// Some gateways, OpenRouter among them, report a failure mid-stream this way.
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<CallDelta>,
    /// OpenRouter's reasoning blocks, streamed in pieces keyed by `index`.
    #[serde(default)]
    reasoning_details: Vec<Value>,
}

#[derive(Deserialize)]
struct CallDelta {
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<FnDelta>,
}

#[derive(Deserialize)]
struct FnDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
    #[serde(default)]
    cost: Option<f64>,
}

impl Chunk {
    fn into_events(self) -> Vec<Result<Event, Error>> {
        if let Some(error) = self.error {
            let message = error["message"]
                .as_str()
                .map_or_else(|| error.to_string(), str::to_string);
            return vec![Err(Error::Other(anyhow!("{message}")))];
        }
        let mut out = Vec::new();
        for choice in self.choices {
            if let Some(text) = choice.delta.content.filter(|t| !t.is_empty()) {
                out.push(Ok(Event::Text(text)));
            }
            for detail in choice.delta.reasoning_details {
                let key = detail["index"]
                    .as_u64()
                    .map(|i| i.to_string())
                    .or_else(|| detail["id"].as_str().map(str::to_string));
                out.push(Ok(Event::Replay {
                    key: key.unwrap_or_default(),
                    part: detail,
                }));
            }
            for call in choice.delta.tool_calls {
                let (name, arguments) = match call.function {
                    Some(f) => (f.name, f.arguments),
                    None => (None, None),
                };
                // Some servers omit the index. The id then tells calls apart, and a fragment with
                // neither gets an empty key, which continues the call opened last.
                let key = call
                    .index
                    .map(|i| i.to_string())
                    .or_else(|| call.id.clone())
                    .unwrap_or_default();
                out.push(Ok(Event::ToolCallDelta {
                    key,
                    id: call.id,
                    name,
                    arguments,
                }));
            }
            // Not `Done`: usage arrives in a later frame, and a reader that stopped here would
            // lose it. `[DONE]` ends the stream; this only marks the turn complete, so a server
            // that omits the sentinel is still distinguishable from a dropped connection.
            if let Some(reason) = choice.finish_reason.as_deref() {
                if reason == "length" {
                    out.push(Ok(Event::Truncated));
                }
                out.push(Ok(Event::Stop));
            }
        }
        if let Some(u) = self.usage {
            let total = if u.total_tokens > 0 {
                u.total_tokens
            } else {
                u.prompt_tokens + u.completion_tokens
            };
            out.push(Ok(Event::Usage(Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: total,
                cost: u.cost,
                ..Usage::default()
            })));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_text_and_tool_fragments() {
        let events = parse_frame(
            r#"{"choices":[{"delta":{"content":"hi","tool_calls":[
               {"index":0,"id":"c1","function":{"name":"read","arguments":"{\"p"}}]}}]}"#,
        );
        let events: Vec<_> = events.into_iter().map(Result::unwrap).collect();
        assert_eq!(events[0], Event::Text("hi".into()));
        assert_eq!(
            events[1],
            Event::ToolCallDelta {
                key: "0".into(),
                id: Some("c1".into()),
                name: Some("read".into()),
                arguments: Some("{\"p".into()),
            }
        );
    }

    #[test]
    fn a_length_stop_is_reported_as_truncation() {
        let events = parse_frame(r#"{"choices":[{"delta":{},"finish_reason":"length"}]}"#);
        assert_eq!(events[0].as_ref().unwrap(), &Event::Truncated);
        assert_eq!(events[1].as_ref().unwrap(), &Event::Stop);
    }

    /// Not every OpenAI-compatible server sends `[DONE]`. The stop reason is the other mark of a
    /// complete turn, and without one the agent cannot tell a finished stream from a dropped one.
    #[test]
    fn a_stop_reason_marks_the_turn_complete() {
        let events = parse_frame(r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#);
        assert_eq!(events[0].as_ref().unwrap(), &Event::Stop);
    }

    #[test]
    fn calls_without_an_index_are_keyed_by_id() {
        let events = parse_frame(
            r#"{"choices":[{"delta":{"tool_calls":[
               {"id":"a","function":{"name":"read","arguments":"{}"}},
               {"id":"b","function":{"name":"read","arguments":"{}"}}]}}]}"#,
        );
        let keys: Vec<_> = events
            .iter()
            .map(|e| match e.as_ref().unwrap() {
                Event::ToolCallDelta { key, .. } => key.clone(),
                other => panic!("expected a tool fragment, got {other:?}"),
            })
            .collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn a_mid_stream_error_is_an_error() {
        let events = parse_frame(r#"{"error":{"message":"upstream overloaded"}}"#);
        let err = events[0].as_ref().unwrap_err().to_string();
        assert!(err.contains("upstream overloaded"), "{err}");
    }

    #[test]
    fn an_empty_assistant_turn_still_has_content() {
        let cfg = Config::for_test("m");
        let body = build_body(&cfg, &[Message::assistant(None, vec![])], &[], "k");
        assert_eq!(body["messages"][0]["content"], "");
    }

    #[test]
    fn only_claude_on_openrouter_asks_for_cache_markers() {
        let body = |provider: &str, model: &str| {
            let cfg = Config {
                provider: provider.into(),
                ..Config::for_test(model)
            };
            build_body(&cfg, &[Message::user("hi")], &[], "k")
        };
        let marker = json!({ "type": "ephemeral" });
        assert_eq!(
            body("openrouter", "anthropic/claude-opus-5.5")["cache_control"],
            marker
        );
        assert!(
            body("openrouter", "openai/gpt-5.6")
                .get("cache_control")
                .is_none()
        );
        assert!(body("ollama", "anthropic/x").get("cache_control").is_none());
    }

    /// OpenRouter streams reasoning in pieces keyed by `index`, and needs the whole array back.
    #[test]
    fn reasoning_details_are_joined_by_index_and_sent_back() {
        let mut assembler = crate::turn::Assembler::new();
        for frame in [
            r#"{"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"a","index":0}]}}]}"#,
            r#"{"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"b","signature":"S","index":0}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        ] {
            for e in parse_frame(frame) {
                assembler.push(e.unwrap());
            }
        }
        let turn = assembler.finish().unwrap();
        let block = json!({"type": "reasoning.text", "text": "ab", "signature": "S", "index": 0});
        assert_eq!(turn.replay, std::slice::from_ref(&block));

        let mut message = Message::assistant(Some("x".into()), vec![]);
        message.replay = turn.replay;
        let body = build_body(&Config::for_test("m"), &[message], &[], "k");
        assert_eq!(body["messages"][0]["reasoning_details"], json!([block]));
    }

    #[test]
    fn done_and_usage_are_recognised() {
        assert_eq!(parse_frame("[DONE]").len(), 1);
        let u = parse_frame(r#"{"usage":{"prompt_tokens":4,"completion_tokens":2}}"#);
        assert_eq!(
            u[0].as_ref().unwrap(),
            &Event::Usage(Usage::from_parts(4, 2))
        );
    }

    #[test]
    fn openrouter_cost_is_carried() {
        let u = parse_frame(r#"{"usage":{"prompt_tokens":4,"completion_tokens":2,"cost":0.25}}"#);
        let Ok(Event::Usage(usage)) = &u[0] else {
            panic!("{u:?}")
        };
        assert_eq!(usage.cost, Some(0.25));
    }
}
