//! `-p --json`: one JSON record per line on stdout, ending in a `result` record.
//!
//! For a program driving minima. Plain `-p` output has no end marker, token counts or error text on
//! stdout, so a caller could only read the exit code. JSON string escaping neutralises escape
//! sequences, so no `printable` pass is needed.

use std::io::Write;
use std::time::Duration;

use serde_json::{Value, json};

use super::Frontend;
use crate::provider::Usage;

pub struct Json<W: Write = std::io::Stdout> {
    out: W,
    /// Assistant text of the provider round-trip in progress.
    text: String,
    /// Text of the last completed round-trip. When the run ends, this is the answer.
    last: String,
    turns: u32,
    input: u64,
    output: u64,
}

impl Default for Json {
    fn default() -> Self {
        Self::new(std::io::stdout())
    }
}

impl<W: Write> Json<W> {
    pub fn new(out: W) -> Self {
        Self {
            out,
            text: String::new(),
            last: String::new(),
            turns: 0,
            input: 0,
            output: 0,
        }
    }

    fn emit(&mut self, record: Value) {
        let _ = writeln!(self.out, "{record}");
        let _ = self.out.flush();
    }

    /// The final record. `outcome` is `complete`, `cancelled` or `error`.
    pub fn result(&mut self, result: &anyhow::Result<()>, cancelled: bool) {
        let (outcome, error) = match result {
            Err(e) => ("error", Some(format!("{e:#}"))),
            Ok(()) if cancelled => ("cancelled", None),
            Ok(()) => ("complete", None),
        };
        let record = json!({
            "type": "result",
            "outcome": outcome,
            "text": self.last,
            "error": error,
            "turns": self.turns,
            "input_tokens": self.input,
            "output_tokens": self.output,
        });
        self.emit(record);
    }
}

impl<W: Write> Frontend for Json<W> {
    fn text(&mut self, delta: &str) {
        self.text.push_str(delta);
    }

    fn tool_start(&mut self, name: &str, arguments: &str) {
        self.emit(json!({"type": "tool_call", "name": name, "arguments": arguments}));
    }

    fn tool_end(&mut self, body: &str, note: Option<&str>, ok: bool) {
        self.emit(json!({"type": "tool_result", "ok": ok, "note": note, "output": body}));
    }

    fn retry(&mut self, attempt: u32, delay: Duration) {
        self.emit(json!({"type": "retry", "attempt": attempt, "delay": delay.as_secs_f64()}));
    }

    fn turn_end(&mut self, usage: Usage) {
        self.turns += 1;
        self.input += u64::from(usage.prompt_tokens);
        self.output += u64::from(usage.completion_tokens);
        self.last = std::mem::take(&mut self.text);
        let record = json!({
            "type": "turn",
            "text": self.last,
            "input_tokens": usage.prompt_tokens,
            "output_tokens": usage.completion_tokens,
        });
        self.emit(record);
    }

    fn cancelled(&mut self) {
        // A cancelled round-trip never reaches `turn_end`; its partial text is not an answer.
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(json: &Json<Vec<u8>>) -> Vec<Value> {
        String::from_utf8(json.out.clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).expect("one JSON value per line"))
            .collect()
    }

    #[test]
    fn a_run_ends_in_a_result_carrying_the_last_turn_and_the_totals() {
        let mut json = Json::new(Vec::new());
        json.text("look");
        json.turn_end(Usage::from_parts(10, 2));
        json.tool_start("bash", r#"{"command":"ls"}"#);
        json.tool_end("a\nb\n", Some("exit 1"), true);
        json.text("the ");
        json.text("answer");
        json.turn_end(Usage::from_parts(30, 5));
        json.result(&Ok(()), false);

        let records = records(&json);
        let kinds: Vec<_> = records
            .iter()
            .map(|r| r["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            ["turn", "tool_call", "tool_result", "turn", "result"]
        );
        assert_eq!(records[2]["note"], "exit 1");
        assert_eq!(
            records[4],
            json!({
                "type": "result", "outcome": "complete", "text": "the answer", "error": null,
                "turns": 2, "input_tokens": 40, "output_tokens": 7,
            })
        );
    }

    #[test]
    fn an_error_is_reported_in_the_result() {
        let mut json = Json::new(Vec::new());
        json.result(&Err(anyhow::anyhow!("stopped after 3 turns")), false);
        let result = &records(&json)[0];
        assert_eq!(result["outcome"], "error");
        assert_eq!(result["error"], "stopped after 3 turns");
    }

    #[test]
    fn a_cancel_drops_the_partial_text() {
        let mut json = Json::new(Vec::new());
        json.text("half");
        json.cancelled();
        json.result(&Ok(()), true);
        let result = &records(&json)[0];
        assert_eq!(result["outcome"], "cancelled");
        assert_eq!(result["text"], "");
    }

    #[test]
    fn escape_sequences_stay_escaped() {
        let mut json = Json::new(Vec::new());
        json.text("a\x1b]52;c;aGk=\x07b");
        json.turn_end(Usage::default());
        let raw = String::from_utf8(json.out.clone()).unwrap();
        assert!(!raw.contains('\x1b'), "{raw:?}");
    }
}
