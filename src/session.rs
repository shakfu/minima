//! Saved conversations, for `--continue` and `--resume`. Design: `docs/dev/session-resume.md`.
//!
//! One JSON file per session under `<config dir>/sessions/<root hash>/`, rewritten whole after
//! each prompt and `/compact`. The system message is not stored: it is rebuilt on resume, so an
//! edited `AGENTS.md` applies.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::provider::{Message, Role};

const SCHEMA: u32 = 1;
/// Sessions kept per root. Older files are deleted on write, so old tool output does not pile up.
const KEEP: usize = 10;

#[derive(Serialize, Deserialize)]
struct File {
    schema: u32,
    id: String,
    root: PathBuf,
    provider: String,
    model: String,
    messages: Vec<Message>,
}

/// A conversation as loaded, before the agent takes it.
pub struct Saved {
    pub provider: String,
    pub model: String,
    messages: Vec<Message>,
}

impl Saved {
    /// The messages to resume under `provider` and `model`. A call with no result gets one, since
    /// every dialect rejects an unanswered call. `replay` is cleared on any provider or model
    /// change: it may hold reasoning signed for the other model, and clearing it is always valid.
    pub fn restore(self, provider: &str, model: &str) -> Vec<Message> {
        let mut messages = answer_every_call(self.messages);
        if provider != self.provider || model != self.model {
            for m in &mut messages {
                m.replay.clear();
            }
        }
        messages
    }
}

pub struct Session {
    pub id: String,
    pub path: PathBuf,
    root: PathBuf,
    /// The file exists, so the session can be resumed.
    pub saved: bool,
}

impl Session {
    /// A new session for `root`, stored under `config`, the config directory.
    pub fn new(config: &Path, root: &Path) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let id = format!("{now}-{}", std::process::id());
        Self {
            path: dir(config, root).join(format!("{id}.json")),
            id,
            root: root.to_path_buf(),
            saved: false,
        }
    }

    /// `--resume ID`.
    pub fn open(config: &Path, root: &Path, id: &str) -> Result<(Self, Saved)> {
        // The id becomes a file name, so it cannot be allowed to name another directory.
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            bail!("invalid session id {id:?}");
        }
        Self::load(root, &dir(config, root).join(format!("{id}.json")))
    }

    /// `--continue`: the session for `root` saved most recently.
    pub fn newest(config: &Path, root: &Path) -> Result<(Self, Saved)> {
        match by_age(&dir(config, root)).into_iter().next() {
            Some(path) => Self::load(root, &path),
            None => bail!("no saved session for {}", root.display()),
        }
    }

    fn load(root: &Path, path: &Path) -> Result<(Self, Saved)> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading session {}", path.display()))?;
        let file: File = serde_json::from_str(&raw)
            .with_context(|| format!("parsing session {}", path.display()))?;
        // Refused rather than discarded, as `state.rs` does: a transcript is worth more.
        if file.schema != SCHEMA {
            bail!(
                "session {} has schema {}; this minima reads {SCHEMA}",
                path.display(),
                file.schema
            );
        }
        if file.root != root {
            bail!(
                "session {} belongs to {}, not {}",
                file.id,
                file.root.display(),
                root.display()
            );
        }
        let session = Self {
            id: file.id,
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            saved: true,
        };
        let saved = Saved {
            provider: file.provider,
            model: file.model,
            messages: file.messages,
        };
        Ok((session, saved))
    }

    /// Writes the conversation, without the system message, then prunes older sessions. Nothing
    /// is written while the conversation is empty.
    pub fn save(&mut self, provider: &str, model: &str, messages: &[Message]) -> Result<()> {
        let messages: Vec<Message> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .cloned()
            .collect();
        if messages.is_empty() {
            return Ok(());
        }
        let file = File {
            schema: SCHEMA,
            id: self.id.clone(),
            root: self.root.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            messages,
        };
        let dir = self
            .path
            .parent()
            .context("session path has no directory")?;
        create_private_dir(dir)?;
        let body = serde_json::to_vec(&file)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
        // Tightened before the rename, so the file is never briefly world-readable.
        crate::config::restrict_to_owner(&tmp)?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("writing {}", self.path.display()))?;
        self.saved = true;
        for old in by_age(dir).into_iter().skip(KEEP) {
            let _ = std::fs::remove_file(old);
        }
        Ok(())
    }
}

/// `<config>/sessions/<hash of root>`. A hash, because a path flattened into one name can
/// collide (`/a-b` and `/a/b`) or pass the 255-byte limit.
fn dir(config: &Path, root: &Path) -> PathBuf {
    let hash = fnv1a(root.as_os_str().as_encoded_bytes());
    config.join("sessions").join(format!("{hash:016x}"))
}

/// FNV-1a, 64-bit. std's `DefaultHasher` may change between Rust releases, which would orphan
/// every saved session.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Both levels owner-only: the parent lists which roots have sessions.
fn create_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    crate::config::restrict_to_owner(dir)?;
    if let Some(parent) = dir.parent() {
        crate::config::restrict_to_owner(parent)?;
    }
    Ok(())
}

/// The session files in `dir`, newest first by modification time.
fn by_age(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort_by(|a, b| b.cmp(a));
    files.into_iter().map(|(_, p)| p).collect()
}

/// Inserts a `CANCELLED` result after each assistant turn for every call it left unanswered. A
/// run killed between a call and its result leaves one.
fn answer_every_call(messages: Vec<Message>) -> Vec<Message> {
    let mut out = Vec::with_capacity(messages.len());
    let mut open: Vec<String> = Vec::new();
    let close = |out: &mut Vec<Message>, open: &mut Vec<String>| {
        for id in open.drain(..) {
            out.push(Message::tool_result(id, crate::tools::CANCELLED));
        }
    };
    for m in messages {
        if m.role == Role::Tool {
            open.retain(|id| Some(id) != m.tool_call_id.as_ref());
        } else {
            close(&mut out, &mut open);
            open = m.tool_calls.iter().map(|c| c.id.clone()).collect();
        }
        out.push(m);
    }
    close(&mut out, &mut open);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ToolCall;

    fn call(id: &str) -> ToolCall {
        ToolCall {
            id: id.into(),
            name: "bash".into(),
            arguments: "{}".into(),
        }
    }

    /// A config directory and a root, in a scratch directory dropped with the first.
    fn scratch(name: &str) -> (crate::tools::Scratch, PathBuf, PathBuf) {
        let dir = crate::tools::Scratch::new(name);
        let root = PathBuf::from(dir.file("root"));
        std::fs::create_dir_all(&root).unwrap();
        let config = PathBuf::from(dir.file("config"));
        (dir, config, root)
    }

    fn conversation() -> Vec<Message> {
        let mut answer = Message::assistant(Some("ok".into()), vec![]);
        answer.replay = vec![serde_json::json!({"type": "thinking", "signature": "s"})];
        vec![
            Message::system("sys"),
            Message::user("hi"),
            Message::assistant(None, vec![call("c1")]),
            Message::tool_result("c1", "out"),
            answer,
        ]
    }

    #[test]
    fn round_trips_without_the_system_message_in_an_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, config, root) = scratch("session-round-trip");
        {
            let mut s = Session::new(&config, &root);
            assert!(!s.saved);
            s.save("anthropic", "m", &conversation()).unwrap();
            assert!(s.saved);
            let mode = std::fs::metadata(&s.path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
            let parent = std::fs::metadata(s.path.parent().unwrap()).unwrap();
            assert_eq!(parent.permissions().mode() & 0o777, 0o700);

            let (opened, saved) = Session::open(&config, &root, &s.id).unwrap();
            assert_eq!((opened.id.as_str(), opened.saved), (s.id.as_str(), true));
            assert_eq!(
                (saved.provider.as_str(), saved.model.as_str()),
                ("anthropic", "m")
            );
            let messages = saved.restore("anthropic", "m");
            assert_eq!(messages.len(), 4);
            assert_eq!(messages[0].role, Role::User);
            assert_eq!(messages[3].replay.len(), 1, "same model keeps replay");
        }
    }

    #[test]
    fn an_empty_conversation_writes_nothing() {
        let (_dir, config, root) = scratch("session-empty");
        {
            let mut s = Session::new(&config, &root);
            s.save("p", "m", &[Message::system("sys")]).unwrap();
            assert!(!s.saved && !s.path.exists());
            assert!(Session::newest(&config, &root).is_err());
        }
    }

    #[test]
    fn replay_is_cleared_when_the_provider_or_model_changes() {
        let saved = |messages| Saved {
            provider: "anthropic".into(),
            model: "m".into(),
            messages,
        };
        let replays = |ms: Vec<Message>| ms.iter().map(|m| m.replay.len()).sum::<usize>();
        let rest = conversation().split_off(1);
        assert_eq!(replays(saved(rest.clone()).restore("anthropic", "m")), 1);
        assert_eq!(replays(saved(rest.clone()).restore("anthropic", "m2")), 0);
        assert_eq!(replays(saved(rest).restore("openrouter", "m")), 0);
    }

    #[test]
    fn an_unanswered_call_gets_a_cancelled_result() {
        let messages = vec![
            Message::user("hi"),
            Message::assistant(None, vec![call("a"), call("b")]),
            Message::tool_result("a", "out"),
            Message::user("again"),
            Message::assistant(None, vec![call("c")]),
        ];
        let fixed = answer_every_call(messages);
        let shape: Vec<_> = fixed
            .iter()
            .map(|m| match m.role {
                Role::Tool => format!(
                    "{}={}",
                    m.tool_call_id.as_deref().unwrap(),
                    m.content.as_deref().unwrap()
                ),
                role => format!("{role:?}"),
            })
            .collect();
        let cancelled = crate::tools::CANCELLED;
        assert_eq!(
            shape,
            [
                "User".to_string(),
                "Assistant".into(),
                "a=out".into(),
                format!("b={cancelled}"),
                "User".into(),
                "Assistant".into(),
                format!("c={cancelled}"),
            ]
        );
    }

    #[test]
    fn another_schema_or_root_is_refused_and_named() {
        let (_dir, config, root) = scratch("session-schema");
        {
            let mut s = Session::new(&config, &root);
            s.save("p", "m", &conversation()).unwrap();
            let raw = std::fs::read_to_string(&s.path).unwrap();
            let mut file: serde_json::Value = serde_json::from_str(&raw).unwrap();

            file["schema"] = 2.into();
            std::fs::write(&s.path, file.to_string()).unwrap();
            let err = Session::open(&config, &root, &s.id)
                .err()
                .unwrap()
                .to_string();
            assert!(err.contains("schema 2") && err.contains(&s.id), "{err}");

            file["schema"] = SCHEMA.into();
            file["root"] = "/elsewhere".into();
            std::fs::write(&s.path, file.to_string()).unwrap();
            let err = Session::open(&config, &root, &s.id)
                .err()
                .unwrap()
                .to_string();
            assert!(err.contains("/elsewhere"), "{err}");
        }
    }

    #[test]
    fn an_id_cannot_leave_the_sessions_directory() {
        let (_dir, config, root) = scratch("session-id");
        {
            for id in ["", "../x", "a/b", "a.json"] {
                let err = Session::open(&config, &root, id).err().unwrap().to_string();
                assert!(err.contains("invalid session id"), "{id}: {err}");
            }
        }
    }

    #[test]
    fn continue_takes_the_newest_and_only_ten_are_kept() {
        let (_dir, config, root) = scratch("session-keep");
        {
            for i in 0..(KEEP + 2) {
                let mut s = Session::new(&config, &root);
                s.id = format!("s{i}");
                s.path = s.path.with_file_name(format!("s{i}.json"));
                s.save("p", "m", &conversation()).unwrap();
                // Distinct mtimes, newest last.
                let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000 + i as u64);
                std::fs::File::options()
                    .write(true)
                    .open(&s.path)
                    .unwrap()
                    .set_modified(t)
                    .unwrap();
            }
            let (newest, _) = Session::newest(&config, &root).unwrap();
            assert_eq!(newest.id, format!("s{}", KEEP + 1));
            let left = by_age(newest.path.parent().unwrap());
            assert_eq!(left.len(), KEEP);
            assert!(
                !left
                    .iter()
                    .any(|p| p.ends_with("s0.json") || p.ends_with("s1.json"))
            );
        }
    }

    #[test]
    fn the_root_hash_is_stable() {
        // Pinned: a change here orphans every saved session.
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
