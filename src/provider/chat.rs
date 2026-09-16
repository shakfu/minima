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
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    delta: Delta,
}

#[derive(Deserialize, Default)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<CallDelta>,
}

#[derive(Deserialize)]
struct CallDelta {
    #[serde(default)]
    index: usize,
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
}

impl Chunk {
    fn into_events(self) -> Vec<Result<Event, Error>> {
        let mut out = Vec::new();
        for choice in self.choices {
            if let Some(text) = choice.delta.content.filter(|t| !t.is_empty()) {
                out.push(Ok(Event::Text(text)));
            }
            for call in choice.delta.tool_calls {
                let (name, arguments) = match call.function {
                    Some(f) => (f.name, f.arguments),
                    None => (None, None),
                };
                out.push(Ok(Event::ToolCallDelta {
                    key: call.index.to_string(),
                    id: call.id,
                    name,
                    arguments,
                }));
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
    fn done_and_usage_are_recognised() {
        assert_eq!(parse_frame("[DONE]").len(), 1);
        let u = parse_frame(r#"{"usage":{"prompt_tokens":4,"completion_tokens":2}}"#);
        assert_eq!(
            u[0].as_ref().unwrap(),
            &Event::Usage(Usage::from_parts(4, 2))
        );
    }
}
