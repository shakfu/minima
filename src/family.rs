//! Recursive agents under `-r`/`--recurse`: the family registry, mailboxes, the budget ledger, and
//! the subcommands an agent reaches through `bash`. Design: `docs/dev/recursion.md`.
//!
//! A family lives in one directory, `<state dir>/family/<family id>/`:
//!
//! - `registry.json`: settings the root fixed (root, sandbox, writable set, budget) and members.
//! - `ledger.jsonl`: one line of usage per provider request, by any member.
//! - `<member id>.mail`: messages waiting for that member, one JSON line each.
//! - `<member id>.log`: a spawned child's stdout and stderr.
//! - `lock`: held with `flock` around every read-modify-write of the files above.
//!
//! No daemon: each member reads and writes the files itself.

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::config::{Bounds, Cli};

const SCHEMA: u32 = 1;
/// The share of a budget held back, so a request the projection undercounts still lands inside it.
const RESERVE_PERCENT: u64 = 5;
const ROOT_NAME: &str = "root";
const SUBCOMMANDS: [&str; 5] = ["spawn", "send", "recv", "agents", "shutdown"];

/// Set on every member, so the commands its `bash` runs know whose they are.
const ENV_FAMILY: &str = "MINIMA_FAMILY";
const ENV_SELF: &str = "MINIMA_SELF";
/// Set by `spawn` on the child alone; the child reads and removes them.
const ENV_PARENT: &str = "MINIMA_PARENT";
const ENV_ID: &str = "MINIMA_ID";
const ENV_NAME: &str = "MINIMA_NAME";

#[derive(Debug, Serialize, Deserialize)]
struct Registry {
    schema: u32,
    root: PathBuf,
    sandbox: bool,
    writable: Vec<PathBuf>,
    budget_tokens: Option<u64>,
    budget_cost: Option<f64>,
    members: Vec<Member>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Member {
    id: String,
    name: String,
    parent: Option<String>,
    pid: u32,
    provider: String,
    model: String,
    started_at: u64,
    ended_at: Option<u64>,
}

impl Member {
    fn running(&self) -> bool {
        self.ended_at.is_none() && alive(self.pid)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mail {
    pub from: String,
    pub from_name: String,
    pub at: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Spend {
    member: String,
    at: u64,
    tokens: u64,
    cost: f64,
}

/// This process's place in its family.
#[derive(Debug, Clone)]
pub struct Family {
    dir: PathBuf,
    pub id: String,
    pub me: String,
    pub name: String,
    pub parent: Option<String>,
    budget_tokens: Option<u64>,
    budget_cost: Option<f64>,
}

/// A child's identity, as `spawn` passed it.
pub struct Inherited {
    family: String,
    me: String,
    name: String,
    parent: String,
}

/// `$XDG_STATE_HOME/minima`, else `$HOME/.local/state/minima`.
fn state_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(x).join("minima"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state/minima"))
}

fn families() -> Result<PathBuf> {
    state_dir()
        .map(|d| d.join("family"))
        .context("no state directory for --recurse: set $HOME or $XDG_STATE_HOME")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The same shape as a session id, so a child's session can be resumed by its member id.
fn new_id() -> String {
    format!("{}-{}", now(), std::process::id())
}

/// Released when dropped, by closing the file.
struct Lock(#[allow(dead_code)] std::fs::File);

fn lock(dir: &Path) -> Result<Lock> {
    let path = dir.join("lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    loop {
        // SAFETY: flock takes no pointers; the descriptor is open for the call.
        if unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&file), libc::LOCK_EX) } == 0 {
            return Ok(Lock(file));
        }
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::Interrupted {
            return Err(e).with_context(|| format!("locking {}", path.display()));
        }
    }
}

fn load(dir: &Path) -> Result<Registry> {
    let path = dir.join("registry.json");
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let registry: Registry =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    if registry.schema != SCHEMA {
        bail!(
            "{} has schema {}; this minima reads {SCHEMA}",
            path.display(),
            registry.schema
        );
    }
    Ok(registry)
}

fn store(dir: &Path, registry: &Registry) -> Result<()> {
    let path = dir.join("registry.json");
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(registry)?)
        .with_context(|| format!("writing {}", tmp.display()))?;
    crate::config::restrict_to_owner(&tmp)?;
    std::fs::rename(&tmp, &path).with_context(|| format!("writing {}", path.display()))
}

fn update<T>(dir: &Path, f: impl FnOnce(&mut Registry) -> Result<T>) -> Result<T> {
    let _lock = lock(dir)?;
    let mut registry = load(dir)?;
    let out = f(&mut registry)?;
    store(dir, &registry)?;
    Ok(out)
}

fn append(path: &Path, line: &impl Serialize) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    crate::config::restrict_to_owner(path)?;
    let mut body = serde_json::to_vec(line)?;
    body.push(b'\n');
    file.write_all(&body)
        .with_context(|| format!("writing {}", path.display()))
}

fn mailbox(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.mail"))
}

/// Takes every message waiting for `id`, oldest first, and empties the mailbox.
fn take_mail(dir: &Path, id: &str) -> Result<Vec<Mail>> {
    let _lock = lock(dir)?;
    let path = mailbox(dir, id);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    std::fs::write(&path, "").with_context(|| format!("emptying {}", path.display()))?;
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("parsing {}", path.display())))
        .collect()
}

/// Tokens and USD spent by the whole family so far.
fn spent(dir: &Path) -> Result<(u64, f64)> {
    let path = dir.join("ledger.jsonl");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((0, 0.0)),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .try_fold((0, 0.0), |(tokens, cost), l| {
            let s: Spend =
                serde_json::from_str(l).with_context(|| format!("parsing {}", path.display()))?;
            Ok((tokens + s.tokens, cost + s.cost))
        })
}

/// A pid of a process that still exists. EPERM means it exists under another user.
fn alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: kill with signal 0 takes no pointers and sends nothing.
    let sent = unsafe { libc::kill(pid, 0) } == 0;
    sent || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Guards against a pid the registry still lists but the system has given to another program.
fn is_minima(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
    #[cfg(not(target_os = "linux"))]
    let name = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    Path::new(name.trim())
        .file_name()
        .is_some_and(|n| n == "minima")
}

/// The member's process group if it leads one, as every spawned child does, else the process.
fn signal(pid: u32, sig: libc::c_int) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    // SAFETY: getpgid, killpg and kill take no pointers.
    unsafe {
        if libc::getpgid(pid) == pid {
            libc::killpg(pid, sig);
        } else {
            libc::kill(pid, sig);
        }
    }
}

/// Reads a spawned child's identity and removes it from the environment, so a minima the child's
/// own `bash` starts does not take it for itself. Then applies the settings the root fixed: the
/// root, the sandbox and the writable set, verbatim. Call before any thread starts.
pub fn inherit(cli: &mut Cli) -> Result<Option<Inherited>> {
    let (Some(family), Some(me), Some(name), Some(parent)) = (
        std::env::var(ENV_FAMILY).ok(),
        std::env::var(ENV_ID).ok(),
        std::env::var(ENV_NAME).ok(),
        std::env::var(ENV_PARENT).ok(),
    ) else {
        return Ok(None);
    };
    // SAFETY: called from `start` before the runtime or any other thread exists.
    unsafe {
        std::env::remove_var(ENV_ID);
        std::env::remove_var(ENV_NAME);
        std::env::remove_var(ENV_PARENT);
    }
    if cli.budget_tokens.is_some() || cli.budget_cost.is_some() {
        bail!("a budget is set by the family's root; a spawned agent cannot set its own");
    }
    let registry = load(&families()?.join(&family))?;
    cli.root = Some(registry.root);
    cli.sandbox = registry.sandbox;
    cli.writable = registry.writable;
    Ok(Some(Inherited {
        family,
        me,
        name,
        parent,
    }))
}

/// Joins the family `inherited` names, or founds one with this process as its root. Under
/// `--sandbox`, the family directory joins the writable set: the subcommands run under the
/// sandboxed `bash` and write there. Call before any thread starts: it sets the environment
/// every `bash` command inherits.
pub fn join(inherited: Option<Inherited>, bounds: &mut Bounds, cli: &Cli) -> Result<Family> {
    let pid = std::process::id();
    let family = match inherited {
        Some(child) => {
            let dir = families()?.join(&child.family);
            let registry = update(&dir, |registry| {
                if let Some(m) = registry.members.iter_mut().find(|m| m.id == child.me) {
                    m.pid = pid;
                    m.ended_at = None;
                }
                Ok((registry.budget_tokens, registry.budget_cost))
            })?;
            Family {
                dir,
                id: child.family,
                me: child.me,
                name: child.name,
                parent: Some(child.parent),
                budget_tokens: registry.0,
                budget_cost: registry.1,
            }
        }
        None => {
            let id = new_id();
            let dir = families()?.join(&id);
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            crate::config::restrict_to_owner(&dir)?;
            if let Some(parent) = dir.parent() {
                crate::config::restrict_to_owner(parent)?;
            }
            let registry = Registry {
                schema: SCHEMA,
                root: bounds.root.clone(),
                sandbox: bounds.sandbox,
                writable: bounds.writable.clone(),
                budget_tokens: cli.budget_tokens,
                budget_cost: cli.budget_cost,
                members: vec![Member {
                    id: id.clone(),
                    name: ROOT_NAME.into(),
                    parent: None,
                    pid,
                    provider: String::new(),
                    model: String::new(),
                    started_at: now(),
                    ended_at: None,
                }],
            };
            let _lock = lock(&dir)?;
            store(&dir, &registry)?;
            Family {
                dir,
                me: id.clone(),
                id,
                name: ROOT_NAME.into(),
                parent: None,
                budget_tokens: cli.budget_tokens,
                budget_cost: cli.budget_cost,
            }
        }
    };
    if bounds.sandbox {
        bounds.writable.push(std::fs::canonicalize(&family.dir)?);
    }
    // SAFETY: called from `start` before the runtime or any other thread exists.
    unsafe {
        std::env::set_var(ENV_FAMILY, &family.id);
        std::env::set_var(ENV_SELF, &family.me);
    }
    Ok(family)
}

impl Family {
    /// Records the provider and model, which `spawn` gives a child that names none.
    pub fn describe(&self, provider: &str, model: &str) -> Result<()> {
        update(&self.dir, |r| {
            if let Some(m) = r.members.iter_mut().find(|m| m.id == self.me) {
                m.provider = provider.into();
                m.model = model.into();
            }
            Ok(())
        })
    }

    /// Marks this member ended. Best effort: it runs on the way out.
    pub fn leave(&self) {
        let _ = update(&self.dir, |r| {
            if let Some(m) = r.members.iter_mut().find(|m| m.id == self.me) {
                m.ended_at.get_or_insert_with(now);
            }
            Ok(())
        });
    }

    pub fn take_mail(&self) -> Result<Vec<Mail>> {
        take_mail(&self.dir, &self.me)
    }

    /// A spawned child's final answer, for its parent.
    pub fn answer_parent(&self, text: &str) -> Result<()> {
        let Some(parent) = &self.parent else {
            return Ok(());
        };
        deliver(&self.dir, &self.me, parent, text)
    }

    /// Fails when the family's spend plus the next request could cross a budget less its reserve.
    pub fn check_budget(&self, next_tokens: u64, next_cost: f64) -> Result<()> {
        if self.budget_tokens.is_none() && self.budget_cost.is_none() {
            return Ok(());
        }
        let (tokens, cost) = spent(&self.dir)?;
        if let Some(budget) = self.budget_tokens {
            let line = budget * (100 - RESERVE_PERCENT) / 100;
            if tokens + next_tokens > line {
                bail!(
                    "family token budget: {tokens} of {budget} spent, and the next request could \
                     take {next_tokens} more; the family is shut down"
                );
            }
        }
        if let Some(budget) = self.budget_cost {
            let line = budget * (100 - RESERVE_PERCENT) as f64 / 100.0;
            if cost + next_cost > line {
                bail!(
                    "family cost budget: ${cost:.4} of ${budget:.4} spent, and the next request \
                     could cost ${next_cost:.4} more; the family is shut down"
                );
            }
        }
        Ok(())
    }

    /// Records one request's usage. Skipped without a budget, which nothing else reads it for.
    pub fn spend(&self, tokens: u64, cost: f64) -> Result<()> {
        if self.budget_tokens.is_none() && self.budget_cost.is_none() {
            return Ok(());
        }
        let _lock = lock(&self.dir)?;
        append(
            &self.dir.join("ledger.jsonl"),
            &Spend {
                member: self.me.clone(),
                at: now(),
                tokens,
                cost,
            },
        )
    }

    /// Stops every other member. This one is left to report why and refuse further requests.
    pub fn shutdown_others(&self) -> Result<()> {
        shutdown(&self.dir, Duration::from_secs(5)).map(|_| ())
    }

    /// The system prompt's account of the family and the commands that reach it.
    pub fn prompt(&self) -> String {
        let exe =
            std::env::current_exe().map_or_else(|_| "minima".into(), |p| p.display().to_string());
        let parent = match &self.parent {
            Some(p) => format!(", spawned by {p}"),
            None => String::new(),
        };
        format!(
            "You are agent \"{name}\" (id {me}{parent}) in a family of minima agents. Through bash:\n\
             - `{exe} -r spawn --name NAME -- \"TASK\"` starts a child agent on a self-contained \
             task and prints its id. It runs once, with its own context, and its final answer \
             arrives in your mailbox.\n\
             - `{exe} -r recv --wait SECS` prints messages sent to you, waiting up to SECS for one.\n\
             - `{exe} -r send --to NAME_OR_ID \"TEXT\"` messages a running member.\n\
             - `{exe} -r agents` lists the family.\n\
             - `{exe} -r shutdown` stops every agent in the family.\n\
             Messages that arrive between prompts are put before the next prompt. Children share \
             your root, sandbox and budget.",
            name = self.name,
            me = self.me,
        )
    }
}

/// Puts `text` in `to`'s mailbox, from `from`. Fails when `to` is not running, so nothing is
/// dropped unseen.
fn deliver(dir: &Path, from: &str, to: &str, text: &str) -> Result<()> {
    let _lock = lock(dir)?;
    let registry = load(dir)?;
    let target = resolve(&registry, to)?;
    if !target.running() {
        bail!("{} ({}) is not running", target.name, target.id);
    }
    let from_name = registry
        .members
        .iter()
        .find(|m| m.id == from)
        .map_or_else(|| from.to_string(), |m| m.name.clone());
    append(
        &mailbox(dir, &target.id),
        &Mail {
            from: from.into(),
            from_name,
            at: now(),
            text: text.into(),
        },
    )
}

/// By id, else by the name of a running member. Names are unique among siblings only.
fn resolve<'a>(registry: &'a Registry, to: &str) -> Result<&'a Member> {
    if let Some(m) = registry.members.iter().find(|m| m.id == to) {
        return Ok(m);
    }
    let named: Vec<_> = registry.members.iter().filter(|m| m.name == to).collect();
    let running: Vec<_> = named.iter().filter(|m| m.running()).collect();
    match (running.as_slice(), named.as_slice()) {
        ([one], _) => Ok(one),
        ([], [one]) => Ok(one),
        ([], []) => bail!("no member named {to:?}; `minima -r agents` lists them"),
        _ => bail!("more than one member is named {to:?}; use its id"),
    }
}

/// SIGTERM to every running member but this process, SIGKILL after `grace`, then prune. Returns
/// how many were signalled.
fn shutdown(dir: &Path, grace: Duration) -> Result<usize> {
    let me = std::process::id();
    let targets: Vec<Member> = {
        let _lock = lock(dir)?;
        load(dir)?
            .members
            .into_iter()
            .filter(|m| m.pid != me && m.running())
            .collect()
    };
    let mut signalled = Vec::new();
    for m in &targets {
        if is_minima(m.pid) {
            signal(m.pid, libc::SIGTERM);
            signalled.push(m.pid);
        } else {
            eprintln!(
                "minima: skipped {} ({}): pid {} is no longer minima",
                m.name, m.id, m.pid
            );
        }
    }
    // Not under the lock: a member's exit hook takes it to record its end.
    let deadline = Instant::now() + grace;
    while signalled.iter().any(|&p| alive(p)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    let survivors: Vec<u32> = signalled.iter().copied().filter(|&p| alive(p)).collect();
    for &pid in &survivors {
        signal(pid, libc::SIGKILL);
    }
    std::thread::sleep(Duration::from_millis(100));
    if let Some(pid) = survivors.iter().find(|&&p| alive(p)) {
        bail!("pid {pid} survived SIGKILL");
    }
    let ids: Vec<String> = targets.iter().map(|m| m.id.clone()).collect();
    update(dir, |r| {
        for m in r.members.iter_mut().filter(|m| ids.contains(&m.id)) {
            m.ended_at.get_or_insert_with(now);
        }
        Ok(())
    })?;
    prune(dir)?;
    Ok(signalled.len())
}

/// Drops ended members with their mail and logs. A family with none left is removed whole,
/// ledger included.
fn prune(dir: &Path) -> Result<usize> {
    let (pruned, empty) = update(dir, |r| {
        let (gone, kept): (Vec<Member>, Vec<Member>) =
            r.members.drain(..).partition(|m| !m.running());
        r.members = kept;
        for m in &gone {
            let _ = std::fs::remove_file(mailbox(dir, &m.id));
            let _ = std::fs::remove_file(dir.join(format!("{}.log", m.id)));
        }
        Ok((gone.len(), r.members.is_empty()))
    })?;
    if empty {
        std::fs::remove_dir_all(dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(pruned)
}

#[derive(Parser)]
#[command(name = "minima -r", max_term_width = 80)]
struct Sub {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start a child agent on TASK. Prints its id.
    Spawn {
        /// Unique among this agent's running children.
        #[arg(long)]
        name: String,
        /// Default: this agent's provider.
        #[arg(short = 'P', long, value_name = "ID")]
        provider: Option<String>,
        /// Default: this agent's model, when the provider is too.
        #[arg(short, long, value_name = "ID")]
        model: Option<String>,
        /// Replay a scripted JSON stream instead of the network.
        #[arg(long, value_name = "PATH")]
        mock: Option<PathBuf>,
        #[arg(required = true, num_args = 1.., value_name = "TASK")]
        task: Vec<String>,
    },
    /// Message a running member.
    Send {
        #[arg(long, value_name = "NAME_OR_ID")]
        to: String,
        #[arg(required = true, num_args = 1.., value_name = "TEXT")]
        text: Vec<String>,
    },
    /// Print messages sent to this agent.
    Recv {
        /// Wait up to SECS for a message when none is waiting.
        #[arg(long, value_name = "SECS", default_value_t = 0)]
        wait: u64,
    },
    /// List the family.
    Agents {
        /// The family to list. Default: this agent's, else the newest for this directory.
        #[arg(long, value_name = "ID")]
        family: Option<String>,
        /// Drop ended members, with their mail and logs.
        #[arg(long)]
        prune: bool,
    },
    /// Stop every agent in the family: SIGTERM, then SIGKILL after the grace period.
    Shutdown {
        /// The family to stop. Default: this agent's, else the newest for this directory.
        #[arg(long, value_name = "ID")]
        family: Option<String>,
        #[arg(long, value_name = "SECS", default_value_t = 5)]
        grace: u64,
    },
}

/// Runs a subcommand when the arguments are `-r SUBCOMMAND ...`, else returns None. Only directly
/// after `-r`, so a prompt that starts with one of the names still reaches the agent elsewhere.
pub fn dispatch(args: &[OsString]) -> Result<Option<ExitCode>> {
    let (Some(flag), Some(name)) = (args.get(1), args.get(2)) else {
        return Ok(None);
    };
    if !(flag == "-r" || flag == "--recurse") || !SUBCOMMANDS.iter().any(|s| name == *s) {
        return Ok(None);
    }
    let sub = Sub::parse_from(
        std::iter::once(OsString::from("minima -r")).chain(args[2..].iter().cloned()),
    );
    let base = families()?;
    match sub.command {
        Cmd::Spawn {
            name,
            provider,
            model,
            mock,
            task,
        } => {
            let (family, me) = own()?;
            let id = spawn(
                &base.join(family),
                &me,
                &name,
                provider,
                model,
                mock,
                &task.join(" "),
            )?;
            println!("{id}");
        }
        Cmd::Send { to, text } => {
            let (family, me) = own()?;
            deliver(&base.join(family), &me, &to, &text.join(" "))?;
        }
        Cmd::Recv { wait } => {
            let (family, me) = own()?;
            let dir = base.join(family);
            let deadline = Instant::now() + Duration::from_secs(wait);
            let mail = loop {
                let mail = take_mail(&dir, &me)?;
                if !mail.is_empty() || Instant::now() >= deadline {
                    break mail;
                }
                std::thread::sleep(Duration::from_millis(500));
            };
            if mail.is_empty() {
                println!("no messages");
            }
            for m in mail {
                println!("from {} ({}):\n{}\n", m.from_name, m.from, m.text);
            }
        }
        Cmd::Agents {
            family,
            prune: true,
        } => {
            let dir = base.join(find(&base, family)?);
            println!("pruned {}", prune(&dir)?);
        }
        Cmd::Agents {
            family,
            prune: false,
        } => {
            let dir = base.join(find(&base, family)?);
            print!(
                "{}",
                table(&load(&dir)?, std::env::var(ENV_SELF).ok().as_deref())
            );
        }
        Cmd::Shutdown { family, grace } => {
            let dir = base.join(find(&base, family)?);
            let n = shutdown(&dir, Duration::from_secs(grace))?;
            println!("stopped {n}");
        }
    }
    Ok(Some(ExitCode::SUCCESS))
}

/// The family and member the calling `bash` belongs to.
fn own() -> Result<(String, String)> {
    match (std::env::var(ENV_FAMILY), std::env::var(ENV_SELF)) {
        (Ok(family), Ok(me)) => Ok((family, me)),
        _ => bail!("not inside a --recurse agent: ${ENV_FAMILY} and ${ENV_SELF} are not set"),
    }
}

/// `--family`, else the caller's own, else the newest family rooted at the current directory.
fn find(base: &Path, named: Option<String>) -> Result<String> {
    if let Some(id) = named.or_else(|| std::env::var(ENV_FAMILY).ok()) {
        return Ok(id);
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let mut found: Vec<(SystemTime, String)> = std::fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let registry = load(&e.path()).ok()?;
            let modified = e
                .path()
                .join("registry.json")
                .metadata()
                .ok()?
                .modified()
                .ok()?;
            (registry.root == cwd).then(|| (modified, e.file_name().to_string_lossy().into_owned()))
        })
        .collect();
    found.sort();
    match found.pop() {
        Some((_, id)) => Ok(id),
        None => bail!("no family rooted at {}; pass --family", cwd.display()),
    }
}

fn spawn(
    dir: &Path,
    me: &str,
    name: &str,
    provider: Option<String>,
    model: Option<String>,
    mock: Option<PathBuf>,
    task: &str,
) -> Result<String> {
    let id = new_id();
    let exe = std::env::current_exe().context("finding the minima executable")?;
    let mock = mock
        .map(std::fs::canonicalize)
        .transpose()
        .context("resolving --mock")?;
    let family = dir
        .file_name()
        .context("family directory has no name")?
        .to_os_string();
    update(dir, |r| {
        if r.members
            .iter()
            .any(|m| m.parent.as_deref() == Some(me) && m.name == name && m.running())
        {
            bail!("a running child of this agent is already named {name:?}");
        }
        let (provider, model) = match (provider, model) {
            (None, model) => {
                let own = r.members.iter().find(|m| m.id == me);
                let provider = own.map(|m| m.provider.clone()).unwrap_or_default();
                let model = model.or_else(|| own.map(|m| m.model.clone()));
                (provider, model.unwrap_or_default())
            }
            (Some(provider), model) => (provider, model.unwrap_or_default()),
        };
        let log_path = dir.join(format!("{id}.log"));
        let log = std::fs::File::create(&log_path)
            .with_context(|| format!("creating {}", log_path.display()))?;
        crate::config::restrict_to_owner(&log_path)?;
        let mut command = Command::new(&exe);
        command.args(["--recurse", "--no-color", "-p", task]);
        match &mock {
            Some(path) => {
                command.arg("--mock").arg(path);
            }
            None => {
                if !provider.is_empty() {
                    command.args(["-P", &provider]);
                }
                if !model.is_empty() {
                    command.args(["-m", &model]);
                }
            }
        }
        if r.sandbox {
            // The session directory is outside the sandbox's writable set.
            command.arg("--no-session");
        }
        command
            .current_dir(&r.root)
            .env(ENV_FAMILY, &family)
            .env(ENV_PARENT, me)
            .env(ENV_ID, &id)
            .env(ENV_NAME, name)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            // Its own group, so a signal to this command's group does not reach it, and
            // `shutdown` can reach it and everything it starts.
            .process_group(0);
        let child = command.spawn().context("starting the child agent")?;
        r.members.push(Member {
            id: id.clone(),
            name: name.into(),
            parent: Some(me.into()),
            pid: child.id(),
            provider,
            model,
            started_at: now(),
            ended_at: None,
        });
        Ok(())
    })?;
    Ok(id)
}

fn table(registry: &Registry, me: Option<&str>) -> String {
    let parent = me.and_then(|me| registry.members.iter().find(|m| m.id == me)?.parent.clone());
    let mut out = String::new();
    for m in &registry.members {
        let role = if Some(m.id.as_str()) == me {
            "self"
        } else if parent.as_deref() == Some(m.id.as_str()) {
            "parent"
        } else if me.is_some() && m.parent.as_deref() == me {
            "child"
        } else if m.parent.is_some() && m.parent == parent {
            "sibling"
        } else {
            "-"
        };
        let status = if m.running() { "running" } else { "exited" };
        out.push_str(&format!(
            "{:<16} {:<8} {:<8} {:<32} {}\n",
            m.name, role, status, m.model, m.id
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minima-family-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn member(id: &str, name: &str, parent: Option<&str>, pid: u32) -> Member {
        Member {
            id: id.into(),
            name: name.into(),
            parent: parent.map(Into::into),
            pid,
            provider: String::new(),
            model: String::new(),
            started_at: 0,
            ended_at: None,
        }
    }

    fn registry(dir: &Path, members: Vec<Member>) {
        let r = Registry {
            schema: SCHEMA,
            root: dir.into(),
            sandbox: false,
            writable: Vec::new(),
            budget_tokens: None,
            budget_cost: None,
            members,
        };
        store(dir, &r).unwrap();
    }

    fn family(dir: &Path, tokens: Option<u64>, cost: Option<f64>) -> Family {
        Family {
            dir: dir.into(),
            id: "f".into(),
            me: "a".into(),
            name: ROOT_NAME.into(),
            parent: None,
            budget_tokens: tokens,
            budget_cost: cost,
        }
    }

    #[test]
    fn mail_is_delivered_in_order_and_taken_once() {
        let dir = scratch("mail");
        let me = std::process::id();
        registry(
            &dir,
            vec![
                member("a", "root", None, me),
                member("b", "kid", Some("a"), me),
            ],
        );
        deliver(&dir, "b", "a", "one").unwrap();
        deliver(&dir, "b", "root", "two").unwrap();
        let mail = take_mail(&dir, "a").unwrap();
        let texts: Vec<_> = mail
            .iter()
            .map(|m| (m.from_name.as_str(), m.text.as_str()))
            .collect();
        assert_eq!(texts, [("kid", "one"), ("kid", "two")]);
        assert!(take_mail(&dir, "a").unwrap().is_empty());
    }

    #[test]
    fn mail_to_an_ended_member_is_refused() {
        let dir = scratch("ended");
        let mut gone = member("b", "kid", Some("a"), std::process::id());
        gone.ended_at = Some(1);
        registry(
            &dir,
            vec![member("a", "root", None, std::process::id()), gone],
        );
        let err = deliver(&dir, "a", "kid", "hi").unwrap_err().to_string();
        assert!(err.contains("not running"), "{err}");
    }

    #[test]
    fn the_budget_holds_back_its_reserve_and_counts_every_member() {
        let dir = scratch("budget");
        let f = family(&dir, Some(1000), None);
        f.spend(600, 0.0).unwrap();
        Family {
            me: "b".into(),
            ..f.clone()
        }
        .spend(200, 0.0)
        .unwrap();
        // 950 is the line: 1000 less 5%.
        f.check_budget(150, 0.0).unwrap();
        assert!(f.check_budget(151, 0.0).is_err());
    }

    #[test]
    fn a_cost_budget_is_checked_apart_from_tokens() {
        let dir = scratch("cost");
        let f = family(&dir, None, Some(1.0));
        f.spend(10, 0.9).unwrap();
        f.check_budget(10, 0.04).unwrap();
        let err = f.check_budget(10, 0.06).unwrap_err().to_string();
        assert!(err.contains("cost budget"), "{err}");
    }

    #[test]
    fn without_a_budget_nothing_is_recorded() {
        let dir = scratch("no-budget");
        let f = family(&dir, None, None);
        f.spend(10, 1.0).unwrap();
        assert!(!dir.join("ledger.jsonl").exists());
        f.check_budget(u64::MAX / 2, 1e9).unwrap();
    }

    #[test]
    fn shutdown_skips_a_pid_that_is_not_minima() {
        let dir = scratch("recycled");
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        registry(&dir, vec![member("b", "kid", Some("a"), sleeper.id())]);
        shutdown(&dir, Duration::from_millis(100)).unwrap();
        assert!(sleeper.try_wait().unwrap().is_none(), "sleep was signalled");
        sleeper.kill().unwrap();
        let _ = sleeper.wait();
    }

    #[test]
    fn prune_drops_ended_members_and_removes_an_empty_family() {
        let dir = scratch("prune");
        let mut gone = member("b", "kid", Some("a"), std::process::id());
        gone.ended_at = Some(1);
        registry(
            &dir,
            vec![member("a", "root", None, std::process::id()), gone],
        );
        std::fs::write(mailbox(&dir, "b"), "").unwrap();
        assert_eq!(prune(&dir).unwrap(), 1);
        assert!(!mailbox(&dir, "b").exists());
        update(&dir, |r| {
            r.members[0].ended_at = Some(2);
            Ok(())
        })
        .unwrap();
        prune(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        registry(&dir, vec![member("a", "root", None, std::process::id())]);
        deliver(&dir, "a", "a", "hi").unwrap();
        for f in ["registry.json", "a.mail"] {
            let mode = std::fs::metadata(dir.join(f)).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{f}");
        }
    }
}
