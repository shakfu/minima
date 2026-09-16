//! Borrowed stream events in, one owned assistant message out. No I/O, no presentation.

use crate::provider::{Event, ToolCall, Usage};

#[derive(Debug, Default, Clone)]
pub struct Turn {
    pub text: String,
    pub calls: Vec<ToolCall>,
    pub usage: Usage,
}

impl Turn {
    pub fn wants_tools(&self) -> bool {
        !self.calls.is_empty()
    }
}

#[derive(Debug, Default, Clone)]
struct Partial {
    key: String,
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
pub struct Assembler {
    text: String,
    /// In the order the model opened them, not the order fragments arrive. A Vec rather than a
    /// map because the keys are dialect-specific strings with no meaningful ordering of their
    /// own, and a turn has a handful of calls at most.
    calls: Vec<Partial>,
    usage: Usage,
    done: bool,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn push(&mut self, event: Event) {
        match event {
            Event::Text(t) => self.text.push_str(&t),
            Event::Usage(u) => self.usage = merge_usage(self.usage, u),
            Event::Done => self.done = true,
            Event::ToolCallDelta {
                key,
                id,
                name,
                arguments,
            } => {
                let slot = match self.calls.iter_mut().position(|c| c.key == key) {
                    Some(i) => &mut self.calls[i],
                    None => {
                        self.calls.push(Partial {
                            key,
                            ..Partial::default()
                        });
                        self.calls.last_mut().expect("just pushed")
                    }
                };
                if let Some(id) = id {
                    slot.id = id;
                }
                if let Some(name) = name {
                    slot.name = name;
                }
                if let Some(args) = arguments {
                    slot.arguments.push_str(&args);
                }
            }
        }
    }

    pub fn finish(self) -> Turn {
        let calls = self
            .calls
            .into_iter()
            .filter(|p| !p.name.is_empty())
            .map(|p| ToolCall {
                id: p.id,
                name: p.name,
                arguments: p.arguments,
            })
            .collect();
        Turn {
            text: self.text,
            calls,
            usage: self.usage,
        }
    }
}

/// Anthropic reports input tokens in `message_start` and the final output count in
/// `message_delta`, so a later frame carrying only one figure must not erase the other.
fn merge_usage(old: Usage, new: Usage) -> Usage {
    let prompt = if new.prompt_tokens > 0 {
        new.prompt_tokens
    } else {
        old.prompt_tokens
    };
    let completion = if new.completion_tokens > 0 {
        new.completion_tokens
    } else {
        old.completion_tokens
    };
    // The total comes from the merged halves, never from the frame that carried only one of
    // them: a message_delta reporting 45 output tokens also reports a total of 45. A provider's
    // own figure is used only when neither half is known.
    let total = if prompt + completion > 0 {
        prompt + completion
    } else {
        new.total_tokens.max(old.total_tokens)
    };
    Usage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(key: &str, id: Option<&str>, name: Option<&str>, args: Option<&str>) -> Event {
        Event::ToolCallDelta {
            key: key.to_string(),
            id: id.map(String::from),
            name: name.map(String::from),
            arguments: args.map(String::from),
        }
    }

    #[test]
    fn concatenates_text() {
        let mut a = Assembler::new();
        a.push(Event::Text("he".into()));
        a.push(Event::Text("llo".into()));
        assert_eq!(a.finish().text, "hello");
    }

    #[test]
    fn joins_argument_fragments() {
        let mut a = Assembler::new();
        a.push(delta("0", Some("c1"), Some("read"), Some("{\"pa")));
        a.push(delta("0", None, None, Some("th\":\"x\"}")));
        let turn = a.finish();
        assert_eq!(turn.calls.len(), 1);
        assert_eq!(turn.calls[0].id, "c1");
        assert_eq!(turn.calls[0].arguments, "{\"path\":\"x\"}");
    }

    /// Keys are dialect-specific and opaque -- a Chat index, a Responses item id, an Anthropic
    /// block index -- so dispatch follows the order the model opened the calls, not any ordering
    /// of the keys themselves.
    #[test]
    fn keeps_the_order_calls_were_opened_in() {
        let mut a = Assembler::new();
        a.push(delta("item_9", Some("b"), Some("write"), Some("{\"x\":1}")));
        a.push(delta("item_2", Some("a"), Some("read"), Some("{}")));
        a.push(delta("item_9", None, None, Some("")));
        let turn = a.finish();
        let names: Vec<_> = turn.calls.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["write", "read"]);
    }

    /// Anthropic reports input tokens in `message_start` and the final output count in
    /// `message_delta`. The second frame must not erase the first.
    #[test]
    fn a_later_partial_usage_frame_does_not_erase_the_other_half() {
        let mut a = Assembler::new();
        a.push(Event::Usage(Usage::from_parts(120, 0)));
        a.push(Event::Usage(Usage::from_parts(0, 45)));
        let usage = a.finish().usage;
        assert_eq!(usage.prompt_tokens, 120);
        assert_eq!(usage.completion_tokens, 45);
        assert_eq!(usage.total_tokens, 165);
    }

    #[test]
    fn drops_calls_that_never_got_a_name() {
        let mut a = Assembler::new();
        a.push(delta("0", Some("c1"), None, Some("{}")));
        assert!(a.finish().calls.is_empty());
    }

    #[test]
    fn records_done_and_usage() {
        let mut a = Assembler::new();
        a.push(Event::Usage(Usage::from_parts(7, 3)));
        a.push(Event::Done);
        assert!(a.is_done());
        assert_eq!(a.finish().usage.total_tokens, 10);
    }
}
