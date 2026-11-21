//! `--acp` as a client sees it: JSON-RPC on the built binary's stdin and stdout.

#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use serde_json::{Value, json};

const LIMIT: Duration = Duration::from_secs(20);

struct Client {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<Value>,
    next: u64,
    /// Every message from the agent that was not the answer to one of our requests.
    seen: Vec<Value>,
    /// The optionId each permission request is answered with; None cancels it.
    permit: Option<&'static str>,
}

impl Client {
    fn start(dir: &Path, script: &Path) -> Self {
        Self::start_with(dir, script, &[])
    }

    fn start_with(dir: &Path, script: &Path, args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_minima"))
            .env("XDG_CONFIG_HOME", dir.join(".config"))
            .current_dir(dir)
            .arg("--acp")
            .arg("--mock")
            .arg(script)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("running minima");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in stdout.lines() {
                let line = line.expect("reading stdout");
                let message = serde_json::from_str(&line).unwrap_or_else(|e| {
                    panic!("stdout carried a line that is not JSON: {line:?}: {e}")
                });
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
            next: 0,
            seen: Vec::new(),
            permit: None,
        }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{message}").expect("writing stdin");
    }

    /// Sends a request and reads until its answer, answering permission requests on the way.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let message = match self.lines.recv_timeout(LIMIT) {
                Ok(message) => message,
                Err(RecvTimeoutError::Timeout) => panic!("no answer to {method} in {LIMIT:?}"),
                Err(RecvTimeoutError::Disconnected) => panic!("minima exited during {method}"),
            };
            assert_eq!(message["jsonrpc"], "2.0", "{message}");
            if message["method"] == "session/request_permission" {
                let outcome = match self.permit {
                    Some(option) => json!({"outcome": "selected", "optionId": option}),
                    None => json!({"outcome": "cancelled"}),
                };
                let reply =
                    json!({"jsonrpc": "2.0", "id": message["id"], "result": {"outcome": outcome}});
                self.send(reply);
            }
            if message.get("method").is_none() && message["id"] == id {
                return message;
            }
            self.seen.push(message);
        }
    }

    fn initialize(&mut self) -> Value {
        self.call(
            "initialize",
            json!({"protocolVersion": 1, "clientCapabilities": {}}),
        )
    }

    fn new_session(&mut self, dir: &Path) -> String {
        let answer = self.call("session/new", json!({"cwd": dir, "mcpServers": []}));
        answer["result"]["sessionId"]
            .as_str()
            .expect("a session id")
            .to_string()
    }

    fn prompt(&mut self, session: &str, text: &str) -> Value {
        let prompt = json!([{"type": "text", "text": text}]);
        self.call(
            "session/prompt",
            json!({"sessionId": session, "prompt": prompt}),
        )
    }

    /// The `sessionUpdate` kinds seen so far, in order.
    fn updates(&self) -> Vec<&Value> {
        self.seen
            .iter()
            .filter(|m| m["method"] == "session/update")
            .map(|m| &m["params"]["update"])
            .collect()
    }

    fn asked(&self) -> usize {
        self.seen
            .iter()
            .filter(|m| m["method"] == "session/request_permission")
            .count()
    }

    fn recv(&self) -> Value {
        self.lines.recv_timeout(LIMIT).expect("a message")
    }

    /// Sends a prompt without waiting for its answer.
    fn start_prompt(&mut self, id: u64, session: &str, text: &str) {
        let prompt = json!([{"type": "text", "text": text}]);
        self.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "session/prompt",
                         "params": {"sessionId": session, "prompt": prompt}}),
        );
    }

    fn cancel(&mut self, session: &str) {
        self.send(
            json!({"jsonrpc": "2.0", "method": "session/cancel", "params": {"sessionId": session}}),
        );
    }

    /// Closing stdin ends the server.
    fn close(mut self) {
        drop(self.stdin);
        let status = self.child.wait().expect("waiting for minima");
        assert!(status.success(), "{status}");
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("minima-acp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    std::fs::canonicalize(dir).unwrap()
}

/// The script `make acp-conformance` in minos runs against.
fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("mock/acp.json")
}

fn code(answer: &Value) -> i64 {
    answer["error"]["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("not an error: {answer}"))
}

#[test]
fn requests_before_initialize_and_unknown_methods_are_refused() {
    let dir = scratch("refused");
    let mut client = Client::start(&dir, &shipped());
    let early = client.call("session/new", json!({"cwd": dir, "mcpServers": []}));
    assert_eq!(code(&early), -32600);
    let init = client.initialize();
    assert_eq!(init["result"]["protocolVersion"], 1);
    assert_eq!(init["result"]["agentCapabilities"]["loadSession"], true);
    assert_eq!(code(&client.call("minos/nothing", json!({}))), -32601);
    assert_eq!(
        code(&client.call("session/new", json!({"cwd": "relative"}))),
        -32602
    );
    let missing = client.prompt("no-such-session", "say hello");
    assert_eq!(code(&missing), -32002);
    client.close();
}

#[test]
fn a_prompt_streams_its_text_and_usage_before_its_stop_reason() {
    let dir = scratch("prompt");
    let mut client = Client::start(&dir, &shipped());
    client.initialize();
    let session = client.new_session(&dir);
    let answer = client.prompt(&session, "say hello");
    assert_eq!(answer["result"]["stopReason"], "end_turn");
    let updates = client.updates();
    assert_eq!(updates[0]["sessionUpdate"], "agent_message_chunk");
    assert_eq!(updates[0]["content"]["text"], "hello from the mock");
    assert_eq!(updates[1]["sessionUpdate"], "usage_update");
    assert_eq!(updates[1]["used"], 126);
    assert_eq!(
        updates[1]["cost"],
        json!({"amount": 0.0001, "currency": "USD"})
    );
    client.close();
}

/// A cancelled request is a denial: the write never runs, and the turn ends normally.
#[test]
fn a_write_runs_only_once_the_client_allows_it() {
    for (permit, written) in [
        (None, false),
        (Some("reject_once"), false),
        (Some("allow_once"), true),
    ] {
        let dir = scratch("permit");
        let mut client = Client::start(&dir, &shipped());
        client.permit = permit;
        client.initialize();
        let session = client.new_session(&dir);
        let answer = client.prompt(&session, "write a note");
        assert_eq!(answer["result"]["stopReason"], "end_turn", "{permit:?}");
        assert_eq!(client.asked(), 1);
        assert_eq!(dir.join("note.txt").exists(), written, "{permit:?}");
        let last = client
            .updates()
            .into_iter()
            .rfind(|u| u["sessionUpdate"] == "tool_call_update")
            .cloned()
            .expect("the call's outcome");
        let status = if written { "completed" } else { "failed" };
        assert_eq!(last["status"], status, "{permit:?}");
        client.close();
    }
}

#[test]
fn allow_always_stops_asking_for_that_tool_in_the_session() {
    let dir = scratch("always");
    let write = |id: &str, path: &str| {
        let arguments = json!({"path": path, "content": "x"}).to_string();
        json!([{"tool_call": {"index": 0, "id": id, "name": "write", "arguments": arguments}}])
    };
    let script = json!({"go": [write("w1", "a.txt"), write("w2", "b.txt"), [{"text": "done"}]]});
    let path = dir.join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    let mut client = Client::start(&dir, &path);
    client.permit = Some("allow_always");
    client.initialize();
    let session = client.new_session(&dir);
    assert_eq!(
        client.prompt(&session, "go")["result"]["stopReason"],
        "end_turn"
    );
    assert_eq!(client.asked(), 1);
    assert!(dir.join("a.txt").exists() && dir.join("b.txt").exists());
    client.close();
}

#[test]
fn session_cancel_ends_a_running_prompt_cancelled() {
    let dir = scratch("cancel");
    let mut client = Client::start(&dir, &shipped());
    client.initialize();
    let session = client.new_session(&dir);
    let prompt = json!([{"type": "text", "text": "count slowly"}]);
    client.send(
        json!({"jsonrpc": "2.0", "id": 99, "method": "session/prompt",
                       "params": {"sessionId": session, "prompt": prompt}}),
    );
    let first = client.lines.recv_timeout(LIMIT).expect("the first chunk");
    assert_eq!(first["params"]["update"]["content"]["text"], "one");
    let started = std::time::Instant::now();
    client.send(
        json!({"jsonrpc": "2.0", "method": "session/cancel", "params": {"sessionId": session}}),
    );
    let answer = client.lines.recv_timeout(LIMIT).expect("the answer");
    assert_eq!(answer["id"], 99);
    assert_eq!(answer["result"]["stopReason"], "cancelled");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    client.close();
}

/// The second process finds the first one's session by id and replays it before answering.
#[test]
fn session_load_replays_the_saved_conversation() {
    let dir = scratch("load");
    let mut first = Client::start(&dir, &shipped());
    first.initialize();
    let session = first.new_session(&dir);
    first.prompt(&session, "say hello");
    first.close();

    let mut second = Client::start(&dir, &shipped());
    second.initialize();
    let load = json!({"sessionId": session, "cwd": dir, "mcpServers": []});
    assert_eq!(second.call("session/load", load)["result"], json!({}));
    let replayed: Vec<_> = second
        .updates()
        .iter()
        .map(|u| (u["sessionUpdate"].clone(), u["content"]["text"].clone()))
        .collect();
    assert_eq!(
        replayed,
        [
            (json!("user_message_chunk"), json!("say hello")),
            // The transcript's text, which keeps the newline the live stream held back.
            (json!("agent_message_chunk"), json!("hello from the mock\n")),
        ]
    );
    let gone = json!({"sessionId": "1-2-3", "cwd": dir, "mcpServers": []});
    assert_eq!(code(&second.call("session/load", gone)), -32002);
    second.close();
}

/// The saved file of the session `id`, wherever its root hash put it.
fn saved(dir: &Path, id: &str) -> Value {
    let sessions = dir.join(".config/minima/sessions");
    for root in std::fs::read_dir(&sessions).expect("a sessions directory") {
        let path = root.unwrap().path().join(format!("{id}.json"));
        if path.exists() {
            return serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        }
    }
    panic!("no saved session {id}");
}

fn script(dir: &Path, script: Value) -> PathBuf {
    let path = dir.join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    path
}

/// A queued prompt used to take the running prompt's place in the cancel table.
#[test]
fn session_cancel_reaches_the_running_prompt_and_those_queued_behind_it() {
    let dir = scratch("cancel-queued");
    let mut client = Client::start(&dir, &shipped());
    client.initialize();
    let session = client.new_session(&dir);
    client.start_prompt(98, &session, "count slowly");
    assert_eq!(client.recv()["params"]["update"]["content"]["text"], "one");
    client.start_prompt(99, &session, "say hello");
    let started = std::time::Instant::now();
    client.cancel(&session);
    for id in [98, 99] {
        let answer = client.recv();
        assert_eq!(answer["id"], id, "{answer}");
        assert_eq!(answer["result"]["stopReason"], "cancelled", "{answer}");
    }
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    client.close();
}

/// With a second prompt queued, the client closing stdin must still stop the first.
#[test]
fn closing_stdin_stops_the_running_prompt() {
    let dir = scratch("eof");
    let mut client = Client::start(&dir, &shipped());
    client.initialize();
    let session = client.new_session(&dir);
    client.start_prompt(98, &session, "count slowly");
    assert_eq!(client.recv()["params"]["update"]["content"]["text"], "one");
    client.start_prompt(99, &session, "say hello");
    let started = std::time::Instant::now();
    client.close();
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
}

/// The root defaults to where minima starts; a session cannot widen it.
#[test]
fn a_session_cwd_must_stay_under_the_root() {
    let dir = scratch("bound");
    let sub = dir.join("sub");
    std::fs::create_dir(&sub).unwrap();
    let mut client = Client::start(&sub, &shipped());
    client.initialize();
    for cwd in [Path::new("/"), dir.as_path()] {
        let new = client.call("session/new", json!({"cwd": cwd, "mcpServers": []}));
        assert_eq!(code(&new), -32602, "{cwd:?}");
        let load = json!({"sessionId": "1-2-3", "cwd": cwd, "mcpServers": []});
        assert_eq!(code(&client.call("session/load", load)), -32602, "{cwd:?}");
    }
    client.new_session(&sub);
    client.close();

    let mut wide = Client::start_with(&sub, &shipped(), &["--root", dir.to_str().unwrap()]);
    wide.initialize();
    wide.new_session(&dir);
    wide.new_session(&sub);
    wide.close();
}

/// usage_update's cost is the session's, so it carries across a load.
#[test]
fn the_cost_carries_across_a_load() {
    let dir = scratch("cost");
    let mut first = Client::start(&dir, &shipped());
    first.initialize();
    let session = first.new_session(&dir);
    first.prompt(&session, "say hello");
    first.close();

    let mut second = Client::start(&dir, &shipped());
    second.initialize();
    let load = json!({"sessionId": session, "cwd": dir, "mcpServers": []});
    second.call("session/load", load);
    second.prompt(&session, "say hello");
    let usage = second
        .updates()
        .into_iter()
        .rfind(|u| u["sessionUpdate"] == "usage_update")
        .cloned()
        .expect("a usage update");
    assert_eq!(usage["cost"], json!({"amount": 0.0002, "currency": "USD"}));
    second.close();
}

/// As `--resume` does, a load takes the saved model where no flag names one.
#[test]
fn a_load_adopts_the_saved_model() {
    let dir = scratch("adopt");
    let mut first = Client::start_with(&dir, &shipped(), &["--model", "m1"]);
    first.initialize();
    let session = first.new_session(&dir);
    first.prompt(&session, "say hello");
    first.close();
    assert_eq!(saved(&dir, &session)["model"], "m1");

    let mut second = Client::start(&dir, &shipped());
    second.initialize();
    let load = json!({"sessionId": session, "cwd": dir, "mcpServers": []});
    second.call("session/load", load);
    second.prompt(&session, "say hello");
    second.close();
    assert_eq!(saved(&dir, &session)["model"], "m1");
}

/// ACP's `refusal`: the refused prompt is left out of the conversation.
#[test]
fn a_refusal_stops_with_refusal_and_drops_the_prompt() {
    let dir = scratch("refusal");
    let path = script(
        &dir,
        json!({"ok": [[{"text": "fine"}]], "bad": [[{"text": "no"}, "refused"]]}),
    );
    let mut client = Client::start(&dir, &path);
    client.initialize();
    let session = client.new_session(&dir);
    assert_eq!(
        client.prompt(&session, "ok")["result"]["stopReason"],
        "end_turn"
    );
    assert_eq!(
        client.prompt(&session, "bad")["result"]["stopReason"],
        "refusal"
    );
    client.close();
    let texts: Vec<_> = saved(&dir, &session)["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["content"].clone())
        .collect();
    assert_eq!(texts, [json!("ok"), json!("fine")]);
}

/// ACP has no `cancelled` status for a tool call, so a cancelled one is `failed`.
#[test]
fn a_cancelled_tool_call_is_reported_failed() {
    let dir = scratch("cancel-tool");
    let arguments = json!({"command": "sleep 10"}).to_string();
    let path = script(
        &dir,
        json!({"go": [[{"tool_call": {"index": 0, "id": "b1", "name": "bash", "arguments": arguments}}]]}),
    );
    let mut client = Client::start(&dir, &path);
    client.initialize();
    let session = client.new_session(&dir);
    client.start_prompt(98, &session, "go");
    loop {
        let message = client.recv();
        if message["method"] == "session/request_permission" {
            let outcome = json!({"outcome": "selected", "optionId": "allow_once"});
            client.send(
                json!({"jsonrpc": "2.0", "id": message["id"], "result": {"outcome": outcome}}),
            );
        }
        if message["params"]["update"]["status"] == "in_progress" {
            break;
        }
    }
    client.cancel(&session);
    let mut last = Value::Null;
    let answer = loop {
        let message = client.recv();
        if message["id"] == 98 {
            break message;
        }
        last = message["params"]["update"].clone();
    };
    assert_eq!(answer["result"]["stopReason"], "cancelled");
    assert_eq!(last["sessionUpdate"], "tool_call_update", "{last}");
    assert_eq!(last["status"], "failed", "{last}");
    client.close();
}

/// Loading a session already open here replays it and keeps it, rather than replacing it.
#[test]
fn loading_an_open_session_replays_it_from_memory() {
    let dir = scratch("load-live");
    let mut client = Client::start(&dir, &shipped());
    client.initialize();
    let session = client.new_session(&dir);
    client.prompt(&session, "say hello");
    client.seen.clear();
    let load = json!({"sessionId": session, "cwd": dir, "mcpServers": []});
    assert_eq!(client.call("session/load", load)["result"], json!({}));
    let kinds: Vec<_> = client
        .updates()
        .iter()
        .map(|u| u["sessionUpdate"].clone())
        .collect();
    assert_eq!(
        kinds,
        [json!("user_message_chunk"), json!("agent_message_chunk")]
    );
    // The mock's second "say hello" turn: the session kept its place.
    client.seen.clear();
    client.prompt(&session, "say hello");
    assert_eq!(
        client.updates()[0]["content"]["text"],
        "hello again from the mock"
    );
    client.close();
}
