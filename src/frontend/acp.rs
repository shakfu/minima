//! `--acp`: the Agent Client Protocol, version 1 (https://agentclientprotocol.com/protocol/).
//!
//! JSON-RPC 2.0, one message per line on stdin and stdout. stdout carries only protocol messages;
//! notes go to stderr. A reader thread acts on `session/cancel` and on the client's responses as
//! they arrive, so both reach a prompt that is running. Every other request waits for the one
//! before it to finish.
//!
//! The client's `fs` and `terminal` capabilities are never used: the tools work on the local
//! disk, as in the other modes. MCP servers are accepted and ignored, since minima has no MCP
//! client. Every tool but `read` asks `session/request_permission` before it runs.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures_util::future::LocalBoxFuture;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use super::{Frontend, describe, printable};
use crate::agent::{Agent, Stop};
use crate::cancel::Cancel;
use crate::provider::{Message, Role, Usage};
use crate::session::Session;

const VERSION: u64 = 1;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
const NOT_FOUND: i64 = -32002;

/// The protocol's side of stdout. One lock per message keeps the reader thread's error replies
/// and the loop's messages from interleaving within a line.
#[derive(Clone)]
pub struct Out(Arc<Mutex<Box<dyn Write + Send>>>);

impl Out {
    pub fn new(writer: impl Write + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(writer))))
    }

    /// serde_json escapes every newline inside a string, so a message is always one line.
    fn send(&self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        let mut out = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    }

    fn reply(&self, id: Value, result: Value) {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "result": result}));
    }

    fn fail(&self, id: Value, code: i64, message: &str) {
        let error = json!({"code": code, "message": message});
        self.send(&json!({"jsonrpc": "2.0", "id": id, "error": error}));
    }

    fn update(&self, session: &str, update: Value) {
        let params = json!({"sessionId": session, "update": update});
        self.send(&json!({"jsonrpc": "2.0", "method": "session/update", "params": params}));
    }
}

/// Requests to the client awaiting its response, by id.
type Waiting = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

/// A request for the loop.
struct Request {
    id: Value,
    method: String,
    params: Value,
    /// For `session/prompt`: the token `session/cancel` sets.
    cancel: Option<Cancel>,
}

/// A JSON-RPC error.
#[derive(Debug)]
struct Fail(i64, String);

fn invalid(message: impl Into<String>) -> Fail {
    Fail(INVALID_PARAMS, message.into())
}

/// Makes the agent for a session rooted at the path. A loaded session passes its saved provider
/// and model, which the agent takes where no flag names them, as `--resume` does.
pub type MakeAgent =
    Box<dyn Fn(PathBuf, Option<(String, String)>) -> LocalBoxFuture<'static, Result<Agent>>>;

/// Serves until the input closes. `store` is the config directory sessions are saved under, or
/// None when they are not saved, which also turns `session/load` off. Every session's `cwd` must
/// be `bound` or under it.
pub async fn serve(
    make: MakeAgent,
    store: Option<PathBuf>,
    bound: PathBuf,
    input: impl BufRead + Send + 'static,
    out: Out,
) -> Result<()> {
    let (sender, mut requests) = mpsc::unbounded_channel();
    let waiting = Waiting::default();
    std::thread::spawn({
        let (out, waiting) = (out.clone(), waiting.clone());
        move || read(input, &out, &waiting, &sender)
    });
    let mut server = Server {
        out,
        waiting,
        next_id: Rc::default(),
        initialized: false,
        sessions: HashMap::new(),
        created: 0,
        store,
        bound,
        make,
    };
    while let Some(request) = requests.recv().await {
        server.handle(request).await;
    }
    Ok(())
}

/// The reader thread. A blocking read on a thread of its own, so a pending read never holds up
/// the runtime's shutdown.
fn read(
    mut input: impl BufRead,
    out: &Out,
    waiting: &Waiting,
    requests: &mpsc::UnboundedSender<Request>,
) {
    let mut prompts = Prompts::default();
    let mut line = Vec::new();
    loop {
        line.clear();
        match input.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&line) else {
            out.fail(Value::Null, PARSE_ERROR, "parse error");
            continue;
        };
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        if !message.is_object() || message["jsonrpc"] != "2.0" {
            out.fail(
                id.unwrap_or_default(),
                INVALID_REQUEST,
                "not a JSON-RPC 2.0 message",
            );
            continue;
        }
        let params = message.get("params").cloned().unwrap_or_default();
        match (method, id) {
            (None, Some(id))
                if message.get("result").is_some() || message.get("error").is_some() =>
            {
                let sender = id.as_u64().and_then(|id| waiting.lock().ok()?.remove(&id));
                if let Some(sender) = sender {
                    let _ = sender.send(message);
                }
            }
            (None, id) => out.fail(id.unwrap_or_default(), INVALID_REQUEST, "no method"),
            (Some("session/cancel"), id) => {
                if let Some(session) = params["sessionId"].as_str() {
                    prompts.cancel(session);
                }
                if let Some(id) = id {
                    out.reply(id, Value::Null);
                }
            }
            // Notifications other than `session/cancel` need no action and get no answer. A
            // prompt sent as one would leave its token in `prompts`.
            (Some(_), None) => {}
            (Some(method), Some(id)) => {
                let cancel =
                    (method == "session/prompt").then(|| match params["sessionId"].as_str() {
                        Some(session) => prompts.add(session),
                        None => Cancel::new(),
                    });
                let request = Request {
                    id,
                    method: method.to_string(),
                    params,
                    cancel,
                };
                if requests.send(request).is_err() {
                    break;
                }
            }
        }
    }
    // The client is gone: nothing it started should keep running, and nothing will answer.
    prompts.cancel_all();
    if let Ok(mut waiting) = waiting.lock() {
        waiting.clear();
    }
}

/// The cancel tokens of every prompt not yet answered, by session, so a cancel reaches the
/// running prompt and those queued behind it. The loop cancels a token once its prompt is
/// answered, which marks it for removal.
#[derive(Default)]
struct Prompts(HashMap<String, Vec<Cancel>>);

impl Prompts {
    fn add(&mut self, session: &str) -> Cancel {
        self.0.retain(|_, tokens| {
            tokens.retain(|c| !c.is_cancelled());
            !tokens.is_empty()
        });
        let cancel = Cancel::new();
        let tokens = self.0.entry(session.to_string()).or_default();
        tokens.push(cancel.clone());
        cancel
    }

    fn cancel(&mut self, session: &str) {
        self.0
            .remove(session)
            .into_iter()
            .flatten()
            .for_each(|c| c.cancel());
    }

    fn cancel_all(&mut self) {
        self.0.drain().flat_map(|(_, t)| t).for_each(|c| c.cancel());
    }
}

struct Server {
    out: Out,
    waiting: Waiting,
    /// Ids of the requests minima sends the client.
    next_id: Rc<Cell<u64>>,
    initialized: bool,
    sessions: HashMap<String, Live>,
    /// Sessions this process has created, for unique ids within one second.
    created: u64,
    store: Option<PathBuf>,
    /// The process's root. No session's `cwd` may leave it.
    bound: PathBuf,
    make: MakeAgent,
}

/// One session's state.
struct Live {
    agent: Agent,
    root: PathBuf,
    file: Option<Session>,
    /// Tool calls so far, for `toolCallId`.
    calls: u64,
    /// Tools the user allowed for the rest of the session.
    allowed: Rc<RefCell<HashSet<String>>>,
    /// USD, summed over the session, when the provider reports it.
    cost: Option<f64>,
}

impl Server {
    async fn handle(&mut self, request: Request) {
        let Request {
            id,
            method,
            params,
            cancel,
        } = request;
        let answer = if !self.initialized && method != "initialize" {
            Err(Fail(INVALID_REQUEST, "initialize first".into()))
        } else {
            match method.as_str() {
                "initialize" => Ok(self.initialize(&params)),
                "session/new" => self.new_session(&params).await,
                "session/load" if self.store.is_some() => self.load_session(&params).await,
                "session/prompt" => {
                    self.prompt(&params, cancel.clone().unwrap_or_default())
                        .await
                }
                _ => Err(Fail(
                    METHOD_NOT_FOUND,
                    format!("method not found: {method}"),
                )),
            }
        };
        match answer {
            Ok(result) => self.out.reply(id, result),
            Err(Fail(code, message)) => self.out.fail(id, code, &message),
        }
        // Answered, so the reader can drop the token; see `Prompts`.
        if let Some(cancel) = cancel {
            cancel.cancel();
        }
    }

    /// Version 1 is the only one, so it is the answer whatever the client asks for.
    fn initialize(&mut self, _params: &Value) -> Value {
        self.initialized = true;
        json!({
            "protocolVersion": VERSION,
            "agentCapabilities": {
                "loadSession": self.store.is_some(),
                "promptCapabilities": {"image": false, "audio": false, "embeddedContext": false},
                "mcpCapabilities": {"http": false, "sse": false},
            },
            "agentInfo": {"name": "minima", "version": env!("CARGO_PKG_VERSION")},
            "authMethods": [],
        })
    }

    async fn new_session(&mut self, params: &Value) -> Result<Value, Fail> {
        let root = root(params, &self.bound)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.created += 1;
        let id = format!("{now}-{}-{}", std::process::id(), self.created);
        let file = self
            .store
            .as_ref()
            .map(|dir| Session::with_id(dir, &root, &id));
        let live = self.start(root, file, None).await?;
        self.sessions.insert(id.clone(), live);
        Ok(json!({"sessionId": id}))
    }

    /// Streams the saved conversation as updates, then answers. A session already open here
    /// replays from memory and keeps its state.
    async fn load_session(&mut self, params: &Value) -> Result<Value, Fail> {
        let id = session_id(params)?;
        let root = root(params, &self.bound)?;
        if let Some(live) = self.sessions.get(id) {
            if live.root != root {
                return Err(invalid(format!(
                    "session {id} is open in {}",
                    live.root.display()
                )));
            }
            replay(&self.out, id, live.agent.messages());
            return Ok(json!({}));
        }
        let store = self
            .store
            .as_ref()
            .expect("load is routed only with a store");
        let (file, saved) = Session::open(store, &root, id).map_err(|e| {
            let missing = e
                .root_cause()
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound);
            let code = if missing { NOT_FOUND } else { INVALID_PARAMS };
            Fail(code, format!("{e:#}"))
        })?;
        let cost = file.cost;
        let pair = (saved.provider.clone(), saved.model.clone());
        let mut live = self.start(root, Some(file), Some(pair)).await?;
        live.cost = cost;
        let messages = saved.restore(live.agent.provider(), live.agent.model());
        replay(&self.out, id, &messages);
        live.agent.resume(messages);
        self.sessions.insert(id.to_string(), live);
        Ok(json!({}))
    }

    /// The system prompt reads `AGENTS.md` from the working directory, so it moves first.
    async fn start(
        &self,
        root: PathBuf,
        file: Option<Session>,
        saved: Option<(String, String)>,
    ) -> Result<Live, Fail> {
        enter(&root)?;
        let agent = (self.make)(root.clone(), saved)
            .await
            .map_err(|e| Fail(INTERNAL_ERROR, format!("{e:#}")))?;
        Ok(Live {
            agent,
            root,
            file,
            calls: 0,
            allowed: Rc::default(),
            cost: None,
        })
    }

    async fn prompt(&mut self, params: &Value, cancel: Cancel) -> Result<Value, Fail> {
        let id = session_id(params)?;
        let text = prompt_text(&params["prompt"])?;
        let live = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| Fail(NOT_FOUND, format!("no session {id}")))?;
        // Cancelled while it waited behind another request.
        if cancel.is_cancelled() {
            return Ok(json!({"stopReason": "cancelled"}));
        }
        enter(&live.root)?;
        let Live {
            agent,
            file,
            calls,
            allowed,
            cost,
            ..
        } = live;
        let mut updates = Updates {
            out: &self.out,
            session: id,
            calls,
            allowed,
            cost,
            size: (!agent.config().context_guessed).then(|| agent.context_window()),
            refused: false,
            asks: Asks {
                out: self.out.clone(),
                waiting: self.waiting.clone(),
                next_id: self.next_id.clone(),
            },
        };
        let result = agent.run(&text, &mut updates, &cancel).await;
        let refused = updates.refused;
        // ACP's `refusal`: the prompt is left out of the next one.
        if refused && !cancel.is_cancelled() {
            agent.forget_last_prompt();
        }
        if let Some(file) = file {
            file.cost = *cost;
            if let Err(e) = file.save(agent.provider(), agent.model(), agent.messages()) {
                eprintln!(
                    "minima: session not saved: {}",
                    printable(&format!("{e:#}"))
                );
            }
        }
        let stop = match result {
            _ if cancel.is_cancelled() => "cancelled",
            Ok(()) if refused => "refusal",
            Ok(()) => "end_turn",
            Err(e) => {
                let stop = match e.downcast_ref::<Stop>() {
                    Some(Stop::MaxTurns(_)) => "max_turn_requests",
                    Some(Stop::Truncated) => "max_tokens",
                    None => return Err(Fail(INTERNAL_ERROR, format!("{e:#}"))),
                };
                eprintln!("minima: {}", printable(&format!("{e:#}")));
                stop
            }
        };
        Ok(json!({"stopReason": stop}))
    }
}

fn enter(root: &Path) -> Result<(), Fail> {
    std::env::set_current_dir(root).map_err(|e| {
        Fail(
            INTERNAL_ERROR,
            format!("changing to {}: {e}", root.display()),
        )
    })
}

fn session_id(params: &Value) -> Result<&str, Fail> {
    params["sessionId"]
        .as_str()
        .ok_or_else(|| invalid("sessionId must be a string"))
}

/// The session's root: `cwd`, which must be an absolute path to a directory at or under `bound`.
/// Also notes any MCP servers, which are accepted and ignored.
fn root(params: &Value, bound: &Path) -> Result<PathBuf, Fail> {
    let cwd = params["cwd"]
        .as_str()
        .map(Path::new)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| invalid("cwd must be an absolute path"))?;
    let root = std::fs::canonicalize(cwd)
        .ok()
        .filter(|p| p.is_dir())
        .ok_or_else(|| invalid(format!("cwd {} is not a directory", cwd.display())))?;
    // Compared resolved, so a symlink cannot lead out.
    if !root.starts_with(bound) {
        return Err(invalid(format!(
            "cwd {} is outside the root {}; start minima with a wider --root",
            root.display(),
            bound.display()
        )));
    }
    if let Some(servers) = params["mcpServers"].as_array().filter(|s| !s.is_empty()) {
        eprintln!(
            "minima: ignoring {} MCP server(s): minima has no MCP client",
            servers.len()
        );
    }
    Ok(root)
}

/// The prompt as one text. A link or an embedded resource becomes its URI, or its text when it
/// carries text; any other block is refused, since none is advertised.
fn prompt_text(blocks: &Value) -> Result<String, Fail> {
    let blocks = blocks
        .as_array()
        .ok_or_else(|| invalid("prompt must be an array of content blocks"))?;
    let mut parts = Vec::new();
    for block in blocks {
        let part = match block["type"].as_str() {
            Some("text") => block["text"].as_str(),
            Some("resource_link") => block["uri"].as_str(),
            Some("resource") => {
                let resource = &block["resource"];
                resource["text"].as_str().or(resource["uri"].as_str())
            }
            other => {
                return Err(invalid(format!(
                    "unsupported content block {}",
                    other.unwrap_or("without a type")
                )));
            }
        };
        parts.push(part.ok_or_else(|| invalid("a content block is missing its text or uri"))?);
    }
    let text = parts.join("\n");
    if text.trim().is_empty() {
        return Err(invalid("the prompt is empty"));
    }
    Ok(text)
}

fn kind(tool: &str) -> &'static str {
    match tool {
        "read" => "read",
        "write" | "edit" => "edit",
        "bash" => "execute",
        _ => "other",
    }
}

/// `read`, `write` or `edit`'s path, made absolute against the working directory.
fn locations(arguments: &Value) -> Value {
    match arguments["path"].as_str() {
        Some(path) => {
            let path = std::env::current_dir().unwrap_or_default().join(path);
            json!([{"path": path}])
        }
        None => json!([]),
    }
}

/// The `toolCall` fields a `tool_call` and a permission request share.
fn tool_call(id: &str, name: &str, arguments: &str) -> Value {
    let raw: Value =
        serde_json::from_str(arguments).unwrap_or_else(|_| Value::String(arguments.into()));
    let title = describe(name, arguments, 120);
    json!({
        "toolCallId": id,
        "title": title.strip_prefix("[tool] ").unwrap_or(&title),
        "kind": kind(name),
        "rawInput": raw,
        "locations": locations(&raw),
    })
}

fn text_chunk(kind: &str, text: &str) -> Value {
    json!({"sessionUpdate": kind, "content": {"type": "text", "text": text}})
}

fn status(id: &str, status: &str) -> Value {
    json!({"sessionUpdate": "tool_call_update", "toolCallId": id, "status": status})
}

/// A loaded session's conversation, as the updates its turns sent.
fn replay(out: &Out, session: &str, messages: &[Message]) {
    for m in messages {
        let text = m.content.as_deref().filter(|t| !t.is_empty());
        match m.role {
            Role::System => {}
            Role::User => {
                if let Some(text) = text {
                    out.update(session, text_chunk("user_message_chunk", text));
                }
            }
            Role::Assistant => {
                if let Some(text) = text {
                    out.update(session, text_chunk("agent_message_chunk", text));
                }
                for call in &m.tool_calls {
                    let mut update = tool_call(&call.id, &call.name, &call.arguments);
                    update["sessionUpdate"] = "tool_call".into();
                    update["status"] = "completed".into();
                    out.update(session, update);
                }
            }
            Role::Tool => {
                if let Some(id) = &m.tool_call_id {
                    let mut update = status(id, "completed");
                    update["content"] = result_content(text.unwrap_or_default());
                    out.update(session, update);
                }
            }
        }
    }
}

fn result_content(body: &str) -> Value {
    json!([{"type": "content", "content": {"type": "text", "text": body}}])
}

/// Sends permission requests and reads their answers.
struct Asks {
    out: Out,
    waiting: Waiting,
    next_id: Rc<Cell<u64>>,
}

impl Asks {
    /// Resolves to the response, or to Null once the client is gone.
    fn ask(&self, method: &str, params: Value) -> LocalBoxFuture<'static, Value> {
        let id = self.next_id.get() + 1;
        self.next_id.set(id);
        let (sender, response) = oneshot::channel();
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.insert(id, sender);
        }
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.out.send(&request);
        let forget = Forget(self.waiting.clone(), id);
        Box::pin(async move {
            let response = response.await.unwrap_or_default();
            drop(forget);
            response
        })
    }
}

/// Drops a request's entry in `Waiting` once nothing awaits its answer, as after a cancel.
struct Forget(Waiting, u64);

impl Drop for Forget {
    fn drop(&mut self) {
        if let Ok(mut waiting) = self.0.lock() {
            waiting.remove(&self.1);
        }
    }
}

const ALLOW_ONCE: &str = "allow_once";
const ALLOW_ALWAYS: &str = "allow_always";
const REJECT_ONCE: &str = "reject_once";

/// The option the client picked. A cancelled request, an error, or an unknown option is None.
fn chosen(response: &Value) -> Option<&str> {
    let outcome = &response["result"]["outcome"];
    if outcome["outcome"] != "selected" {
        return None;
    }
    outcome["optionId"]
        .as_str()
        .filter(|id| [ALLOW_ONCE, ALLOW_ALWAYS, REJECT_ONCE].contains(id))
}

/// One prompt's updates.
struct Updates<'a> {
    out: &'a Out,
    session: &'a str,
    calls: &'a mut u64,
    allowed: &'a Rc<RefCell<HashSet<String>>>,
    cost: &'a mut Option<f64>,
    /// The context window, or None when it is the 128k guess, which a client should not show.
    size: Option<u32>,
    /// The model refused; see `Frontend::refused`.
    refused: bool,
    asks: Asks,
}

impl Updates<'_> {
    /// The call `tool_start` opened. Tools run one at a time.
    fn open_call(&self) -> String {
        format!("minima-{}", self.calls)
    }
}

impl Frontend for Updates<'_> {
    fn text(&mut self, delta: &str) {
        if !delta.is_empty() {
            self.out
                .update(self.session, text_chunk("agent_message_chunk", delta));
        }
    }

    fn tool_start(&mut self, name: &str, arguments: &str) {
        *self.calls += 1;
        let mut update = tool_call(&self.open_call(), name, arguments);
        update["sessionUpdate"] = "tool_call".into();
        update["status"] = "pending".into();
        self.out.update(self.session, update);
    }

    /// `read` changes nothing, so it never asks; the rest can change or delete files.
    fn permit(&mut self, name: &str, arguments: &str) -> Option<LocalBoxFuture<'static, bool>> {
        let id = self.open_call();
        if name == "read" || self.allowed.borrow().contains(name) {
            self.out.update(self.session, status(&id, "in_progress"));
            return None;
        }
        let option = |id: &str, name: &str| json!({"optionId": id, "name": name, "kind": id});
        let params = json!({
            "sessionId": self.session,
            "toolCall": tool_call(&id, name, arguments),
            "options": [
                option(ALLOW_ONCE, "Allow"),
                option(ALLOW_ALWAYS, &format!("Always allow {name} in this session")),
                option(REJECT_ONCE, "Reject"),
            ],
        });
        let response = self.asks.ask("session/request_permission", params);
        let (out, session, allowed, name) = (
            self.out.clone(),
            self.session.to_string(),
            self.allowed.clone(),
            name.to_string(),
        );
        Some(Box::pin(async move {
            let response = response.await;
            let choice = chosen(&response);
            if choice == Some(ALLOW_ALWAYS) {
                allowed.borrow_mut().insert(name);
            }
            let ok = matches!(choice, Some(ALLOW_ONCE | ALLOW_ALWAYS));
            if ok {
                out.update(&session, status(&id, "in_progress"));
            }
            ok
        }))
    }

    /// A cancelled tool returns `CANCELLED` as its output; ACP has no `cancelled` status.
    fn tool_end(&mut self, body: &str, _note: Option<&str>, ok: bool) {
        let ok = ok && body != crate::tools::CANCELLED;
        let mut update = status(&self.open_call(), if ok { "completed" } else { "failed" });
        update["content"] = result_content(body);
        self.out.update(self.session, update);
    }

    fn retry(&mut self, attempt: u32, delay: Duration) {
        eprintln!(
            "minima: retrying ({attempt}) in {:.1}s",
            delay.as_secs_f32()
        );
    }

    /// `used` is the last request's total, which is what the window holds after it.
    fn turn_end(&mut self, usage: Usage) {
        if let Some(cost) = usage.cost {
            *self.cost = Some(self.cost.unwrap_or(0.0) + cost);
        }
        let Some(size) = self.size.filter(|_| usage.total_tokens > 0) else {
            return;
        };
        let mut update = json!({
            "sessionUpdate": "usage_update",
            "used": usage.total_tokens,
            "size": size,
        });
        if let Some(cost) = *self.cost {
            update["cost"] = json!({"amount": cost, "currency": "USD"});
        }
        self.out.update(self.session, update);
    }

    fn compacted(&mut self, before: u32, after: u32) {
        eprintln!("minima: {}", super::compacted(before, after));
    }

    fn cancelled(&mut self) {}

    fn refused(&mut self) {
        self.refused = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prompt_joins_its_text_and_links_and_refuses_the_rest() {
        let text = prompt_text(&json!([
            {"type": "text", "text": "explain"},
            {"type": "resource_link", "uri": "file:///a.rs", "name": "a.rs"},
            {"type": "resource", "resource": {"uri": "file:///b.rs", "text": "fn b() {}"}},
        ]))
        .unwrap();
        assert_eq!(text, "explain\nfile:///a.rs\nfn b() {}");
        for bad in [
            json!([{"type": "image", "data": "", "mimeType": "image/png"}]),
            json!([{"type": "text", "text": "  "}]),
            json!("hello"),
        ] {
            let Fail(code, _) = prompt_text(&bad).expect_err("refused");
            assert_eq!(code, INVALID_PARAMS, "{bad}");
        }
    }

    /// A cancelled or unknown answer is a denial, never approval.
    #[test]
    fn only_a_selected_known_option_is_chosen() {
        let selected =
            |id: &str| json!({"result": {"outcome": {"outcome": "selected", "optionId": id}}});
        assert_eq!(chosen(&selected("allow_once")), Some(ALLOW_ONCE));
        assert_eq!(chosen(&selected("allow_always")), Some(ALLOW_ALWAYS));
        assert_eq!(chosen(&selected("yes")), None);
        assert_eq!(
            chosen(&json!({"result": {"outcome": {"outcome": "cancelled"}}})),
            None
        );
        assert_eq!(
            chosen(&json!({"error": {"code": -32601, "message": "no"}})),
            None
        );
        assert_eq!(chosen(&Value::Null), None);
    }

    #[test]
    fn a_tool_call_names_its_kind_and_target() {
        let call = tool_call("c1", "write", r#"{"path":"/tmp/x.txt","content":"y"}"#);
        assert_eq!(call["kind"], "edit");
        assert_eq!(call["title"], "write /tmp/x.txt");
        assert_eq!(call["locations"], json!([{"path": "/tmp/x.txt"}]));
        assert_eq!(call["rawInput"]["content"], "y");
        let call = tool_call("c2", "bash", "{not json");
        assert_eq!(call["kind"], "execute");
        assert_eq!(call["rawInput"], "{not json");
        assert_eq!(call["locations"], json!([]));
    }

    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn buffered() -> (Out, Arc<Mutex<Vec<u8>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        (Out::new(Shared(buffer.clone())), buffer)
    }

    fn sent(buffer: &Arc<Mutex<Vec<u8>>>) -> Vec<Value> {
        let raw = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        raw.lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn a_message_is_one_line() {
        let (out, buffer) = buffered();
        out.update("s", text_chunk("agent_message_chunk", "a\nb\r\n"));
        let raw = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert_eq!(raw.matches('\n').count(), 1, "{raw:?}");
        assert!(raw.ends_with('\n'));
    }

    /// The title is cut for display; `rawInput` keeps the command whole.
    #[test]
    fn raw_input_carries_the_whole_command() {
        let root = std::env::current_dir().unwrap();
        let command = format!("cd {} && echo {}", root.display(), "x".repeat(300));
        let arguments = json!({"command": command}).to_string();
        let call = tool_call("c1", "bash", &arguments);
        assert_eq!(call["rawInput"]["command"], command);
        assert!(call["title"].as_str().unwrap().len() <= 120, "{call}");
    }

    #[test]
    fn a_cancel_reaches_every_prompt_of_its_session() {
        let mut prompts = Prompts::default();
        let (a, b, other) = (prompts.add("s"), prompts.add("s"), prompts.add("t"));
        prompts.cancel("s");
        assert!(a.is_cancelled() && b.is_cancelled() && !other.is_cancelled());
        prompts.cancel_all();
        assert!(other.is_cancelled());
    }

    /// The loop cancels a token once its prompt is answered, so unknown ids do not pile up.
    #[test]
    fn answered_prompts_are_dropped() {
        let mut prompts = Prompts::default();
        for id in ["x", "y", "z"] {
            prompts.add(id).cancel();
        }
        let _live = prompts.add("s");
        assert_eq!(prompts.0.keys().collect::<Vec<_>>(), ["s"]);
    }

    /// A prompt sent as a notification has no answer to wait for, so it never reaches the loop.
    #[test]
    fn a_prompt_notification_is_ignored() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","method":"session/prompt","params":{"sessionId":"s"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":1,"method":"session/prompt","params":{"sessionId":"s"}}"#,
            "\n",
        );
        let (sender, mut requests) = mpsc::unbounded_channel();
        read(
            input.as_bytes(),
            &Out::new(std::io::sink()),
            &Waiting::default(),
            &sender,
        );
        let request = requests.try_recv().expect("the request with an id");
        assert_eq!(request.id, json!(1));
        assert!(requests.try_recv().is_err(), "the notification was queued");
    }

    #[test]
    fn a_cwd_outside_the_bound_is_refused() {
        let bound = std::fs::canonicalize(std::env::temp_dir()).unwrap();
        let inside = json!({"cwd": bound});
        assert_eq!(root(&inside, &bound).unwrap(), bound);
        for outside in ["/", bound.parent().unwrap().to_str().unwrap()] {
            let Fail(code, _) = root(&json!({"cwd": outside}), &bound).expect_err(outside);
            assert_eq!(code, INVALID_PARAMS);
        }
    }

    #[tokio::test]
    async fn a_dropped_permission_request_forgets_its_entry() {
        let (out, _) = buffered();
        let asks = Asks {
            out,
            waiting: Waiting::default(),
            next_id: Rc::default(),
        };
        let response = asks.ask("session/request_permission", json!({}));
        assert_eq!(asks.waiting.lock().unwrap().len(), 1);
        drop(response);
        assert!(asks.waiting.lock().unwrap().is_empty());
    }

    fn updates<'a>(
        out: &'a Out,
        calls: &'a mut u64,
        allowed: &'a Rc<RefCell<HashSet<String>>>,
        cost: &'a mut Option<f64>,
        size: Option<u32>,
    ) -> Updates<'a> {
        Updates {
            out,
            session: "s",
            calls,
            allowed,
            cost,
            size,
            refused: false,
            asks: Asks {
                out: out.clone(),
                waiting: Waiting::default(),
                next_id: Rc::default(),
            },
        }
    }

    /// A guessed window would show the client a size the model may not have.
    #[test]
    fn usage_goes_out_only_for_a_known_window() {
        let usage = Usage {
            total_tokens: 10,
            cost: Some(0.5),
            ..Usage::default()
        };
        for (size, count) in [(None, 0), (Some(1000), 1)] {
            let (out, buffer) = buffered();
            let (mut calls, allowed, mut cost) = (0, Rc::default(), None);
            updates(&out, &mut calls, &allowed, &mut cost, size).turn_end(usage);
            assert_eq!(sent(&buffer).len(), count, "{size:?}");
            assert_eq!(cost, Some(0.5));
        }
    }

    #[test]
    fn a_cancelled_tool_is_reported_failed() {
        let (out, buffer) = buffered();
        let (mut calls, allowed, mut cost) = (0, Rc::default(), None);
        let mut updates = updates(&out, &mut calls, &allowed, &mut cost, None);
        updates.tool_end(crate::tools::CANCELLED, None, true);
        updates.tool_end("ok", None, true);
        let statuses: Vec<_> = sent(&buffer)
            .iter()
            .map(|m| m["params"]["update"]["status"].clone())
            .collect();
        assert_eq!(statuses, [json!("failed"), json!("completed")]);
    }
}
