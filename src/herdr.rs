//! Agent state reports to Herdr, the terminal multiplexer, when minima runs in one of its panes.
//! See https://herdr.dev/tui/docs/add-herdr-support/.
//!
//! Only the REPL reports. Once the session is saved, each report carries the command that
//! resumes it, so a restored Herdr session reopens the conversation.

use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::Bounds;
use crate::session::Session;

const SOURCE: &str = "minima";
const AGENT: &str = "minima";

/// A report that has not answered by then is killed. Herdr is local, so it is hung, not slow.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Herdr's limits on a resume command. One past them is refused whole.
const MAX_ARGS: usize = 64;
const MAX_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Report {
    Idle,
    Working,
    /// Waiting on the user, with the reason Herdr shows. For an approval prompt, which minima
    /// does not have yet.
    #[cfg_attr(not(test), expect(dead_code, reason = "no approval gate sends it yet"))]
    Blocked(String),
    /// Last: the worker exits after sending it.
    Release,
}

/// How Herdr reopens this session after a restart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resume {
    id: String,
    path: String,
    argv: Vec<String>,
}

impl Resume {
    /// None until the session is saved, or when the command breaks Herdr's rules: the root or a
    /// `--writable` path holds an apostrophe or a control character, or it is too long.
    pub fn of(session: &Session, bounds: &Bounds) -> Option<Self> {
        if !session.saved {
            return None;
        }
        let path = |p: &std::path::Path| p.to_str().map(String::from);
        let mut argv = vec!["minima".into(), "--root".into(), path(&bounds.root)?];
        if bounds.sandbox {
            argv.push("--sandbox".into());
        }
        for dir in &bounds.writable {
            argv.extend(["--writable".into(), path(dir)?]);
        }
        argv.extend(["--resume".into(), session.id.clone()]);
        let bad = |a: &String| a.contains('\'') || a.chars().any(char::is_control);
        let bytes: usize = argv.iter().map(String::len).sum();
        if argv.len() > MAX_ARGS || bytes > MAX_BYTES || argv.iter().any(bad) {
            return None;
        }
        Some(Self {
            id: session.id.clone(),
            path: path(&session.path)?,
            argv,
        })
    }
}

/// The pane minima reports for. `HERDR_PANE_ID`, `HERDR_BIN_PATH` and `HERDR_SOCKET_PATH`.
#[derive(Debug, PartialEq, Eq)]
struct Pane {
    id: String,
    bin: String,
}

impl Pane {
    /// None unless `HERDR_ENV=1` and the pane, binary and socket are all set, as Herdr requires.
    fn from(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let set = |name| var(name).filter(|v| !v.is_empty());
        if var("HERDR_ENV").as_deref() != Some("1") {
            return None;
        }
        set("HERDR_SOCKET_PATH")?;
        Some(Self {
            id: set("HERDR_PANE_ID")?,
            bin: set("HERDR_BIN_PATH")?,
        })
    }

    fn argv(&self, (report, resume): &Msg, seq: u64) -> Vec<String> {
        let mut argv: Vec<String> = vec!["pane".into()];
        let state = |s: &str| ["report-agent", "--state", s].map(String::from);
        match report {
            Report::Idle => argv.extend(state("idle")),
            Report::Working => argv.extend(state("working")),
            Report::Blocked(why) => {
                argv.extend(state("blocked"));
                argv.extend(["--message".into(), why.clone()]);
            }
            Report::Release => argv.push("release-agent".into()),
        }
        argv.extend(
            [
                "--seq",
                &seq.to_string(),
                "--source",
                SOURCE,
                "--agent",
                AGENT,
            ]
            .map(String::from),
        );
        // A release clears the resume command, so it carries none.
        let resume = resume.as_ref().filter(|_| *report != Report::Release);
        if let Some(r) = resume {
            argv.extend(["--agent-session-id".into(), r.id.clone()]);
            argv.extend(["--agent-session-path".into(), r.path.clone()]);
        }
        argv.push(self.id.clone());
        if let Some(r) = resume {
            argv.push("--".into());
            argv.extend(r.argv.iter().cloned());
        }
        argv
    }
}

type Msg = (Report, Option<Resume>);

/// Reports from a worker thread, so a slow or missing `herdr` never stalls the REPL.
pub struct Herdr {
    tx: Sender<Msg>,
    worker: JoinHandle<()>,
}

impl Herdr {
    /// None outside a Herdr pane.
    pub fn from_env() -> Option<Self> {
        let pane = Pane::from(|name| std::env::var(name).ok())?;
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || work(&pane, &rx));
        Some(Self { tx, worker })
    }

    pub fn report(&self, report: Report, resume: Option<Resume>) {
        let _ = self.tx.send((report, resume));
    }

    /// Releases the pane and waits, at most `TIMEOUT`, for the release to be sent.
    pub fn release(self) {
        self.report(Report::Release, None);
        let _ = self.worker.join();
    }
}

fn work(pane: &Pane, rx: &Receiver<Msg>) {
    let mut seq = 0;
    while let Some(msg) = latest(rx) {
        seq = next_seq(seq);
        send(&pane.bin, &pane.argv(&msg, seq));
        if msg.0 == Report::Release {
            return;
        }
    }
}

/// Blocks for one report, then skips to the newest queued one. Herdr shows only the current
/// state, so a stale one would only delay it. None once the sender is gone.
fn latest<T>(rx: &Receiver<T>) -> Option<T> {
    let mut msg = rx.recv().ok()?;
    while let Ok(newer) = rx.try_recv() {
        msg = newer;
    }
    Some(msg)
}

/// Herdr drops a report whose seq is not above the last one it took, across minima restarts too.
/// Milliseconds since the epoch do that, bumped when two reports share a millisecond.
fn next_seq(last: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    now.max(last + 1)
}

/// Failures are ignored: a report is advisory, and the REPL has nowhere to show one.
fn send(bin: &str, argv: &[String]) {
    let Ok(mut child) = Command::new(bin)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            _ => return,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::{Pane, Report, Resume, latest, next_seq};
    use crate::config::Bounds;
    use crate::session::Session;
    use std::path::PathBuf;
    use std::sync::mpsc;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    const FULL: [(&str, &str); 4] = [
        ("HERDR_ENV", "1"),
        ("HERDR_PANE_ID", "p1"),
        ("HERDR_BIN_PATH", "/opt/herdr"),
        ("HERDR_SOCKET_PATH", "/run/herdr.sock"),
    ];

    #[test]
    fn reports_only_with_every_variable_set() {
        assert_eq!(
            Pane::from(env(&FULL)),
            Some(Pane {
                id: "p1".into(),
                bin: "/opt/herdr".into()
            })
        );
        for (name, _) in FULL {
            let partial: Vec<_> = FULL.into_iter().filter(|(k, _)| *k != name).collect();
            assert_eq!(Pane::from(env(&partial)), None, "without {name}");
        }
        let mut off = FULL;
        off[0].1 = "0";
        assert_eq!(Pane::from(env(&off)), None);
        let mut empty = FULL;
        empty[1].1 = "";
        assert_eq!(Pane::from(env(&empty)), None);
    }

    #[test]
    fn argv_matches_the_herdr_cli() {
        let pane = Pane::from(env(&FULL)).unwrap();
        let argv = |report, seq| pane.argv(&(report, None), seq).join(" ");
        let tail = "--source minima --agent minima p1";
        assert_eq!(
            argv(Report::Working, 7),
            format!("pane report-agent --state working --seq 7 {tail}")
        );
        assert_eq!(
            argv(Report::Idle, 8),
            format!("pane report-agent --state idle --seq 8 {tail}")
        );
        assert_eq!(
            argv(Report::Blocked("approve rm -rf".into()), 9),
            format!("pane report-agent --state blocked --message approve rm -rf --seq 9 {tail}")
        );
        assert_eq!(
            argv(Report::Release, 10),
            format!("pane release-agent --seq 10 {tail}")
        );
    }

    fn saved(dir: &crate::tools::Scratch, root: &std::path::Path) -> Session {
        let config = PathBuf::from(dir.file("config"));
        let mut session = Session::new(&config, root);
        let messages = [crate::provider::Message::user("hi")];
        session.save("p", "m", &messages).unwrap();
        session
    }

    #[test]
    fn a_saved_session_is_resumed_with_its_bounds() {
        let dir = crate::tools::Scratch::new("herdr-resume");
        let root = PathBuf::from("/work/x");
        let mut session = saved(&dir, &root);
        let mut bounds = Bounds::new(true, root);
        bounds.writable = vec![PathBuf::from("/opt/a")];
        let resume = Resume::of(&session, &bounds).unwrap();
        let id = session.id.clone();
        assert_eq!(
            resume.argv.join(" "),
            format!("minima --root /work/x --sandbox --writable /opt/a --resume {id}")
        );

        let pane = Pane::from(env(&FULL)).unwrap();
        let argv = pane
            .argv(&(Report::Idle, Some(resume.clone())), 1)
            .join(" ");
        let path = session.path.display();
        assert_eq!(
            argv,
            format!(
                "pane report-agent --state idle --seq 1 --source minima --agent minima \
                 --agent-session-id {id} --agent-session-path {path} p1 -- {}",
                resume.argv.join(" ")
            )
        );
        let release = pane.argv(&(Report::Release, Some(resume)), 2).join(" ");
        assert_eq!(
            release,
            "pane release-agent --seq 2 --source minima --agent minima p1"
        );

        session.saved = false;
        assert_eq!(Resume::of(&session, &bounds), None, "not saved yet");
    }

    #[test]
    fn a_command_herdr_would_refuse_is_not_sent() {
        let dir = crate::tools::Scratch::new("herdr-refuse");
        let session = saved(&dir, "/x".as_ref());
        for root in ["/it's", "/a\nb"] {
            let bounds = Bounds::new(false, PathBuf::from(root));
            assert_eq!(Resume::of(&session, &bounds), None, "{root:?}");
        }
        let mut bounds = Bounds::new(true, PathBuf::from("/x"));
        bounds.writable = vec![PathBuf::from("/w"); 31];
        assert_eq!(Resume::of(&session, &bounds), None, "past 64 arguments");
        bounds.writable = vec![PathBuf::from(format!("/{}", "w".repeat(9000)))];
        assert_eq!(Resume::of(&session, &bounds), None, "past 8 KiB");
    }

    #[test]
    fn only_the_newest_queued_report_is_sent() {
        let (tx, rx) = mpsc::channel();
        for r in [Report::Working, Report::Idle, Report::Working] {
            tx.send(r).unwrap();
        }
        assert_eq!(latest(&rx), Some(Report::Working));
        tx.send(Report::Idle).unwrap();
        assert_eq!(latest(&rx), Some(Report::Idle));
        drop(tx);
        assert_eq!(latest(&rx), None);
    }

    #[test]
    fn seq_always_rises() {
        let a = next_seq(0);
        assert!(a > 1_700_000_000_000, "milliseconds since the epoch: {a}");
        assert_eq!(next_seq(u64::MAX - 1), u64::MAX);
        let b = next_seq(a);
        assert!(b > a);
    }
}
