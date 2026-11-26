//! Recursive agents under `-r`/`--recurse`: the family registry, answers, the budget ledger, and
//! the subcommands an agent reaches through `bash`. Design: `docs/dev/recursion.md`.
//!
//! A family lives in one directory, `<state dir>/family/<family id>/`:
//!
//! - `registry.json`: settings the root fixed (root, sandbox, writable set, budget) and members.
//! - `ledger.jsonl`: one line of usage per provider request, by any member.
//! - `<member id>.answer`: a spawned child's final answer, for its parent's `wait`.
//! - `<member id>.log`: a spawned child's stdout and stderr.
//! - `<member id>.stop`: written by `shutdown`; the member polls for it and exits.
//! - `lock`: held with `flock` around every read-modify-write of the files above.
//!
//! No daemon: each member reads and writes the files itself. A child watches its parent and
//! stops when the parent exits, so no member outlives the agent that started it.

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
const SUBCOMMANDS: [&str; 4] = ["spawn", "wait", "agents", "shutdown"];

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
    budget_tokens: u64,
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
struct Spend {
    member: String,
    at: u64,
    tokens: u64,
    cost: f64,
    /// A reservation for a request not yet answered, counted until a later line settles it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    hold: bool,
    /// The line number of the hold whose request this line records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    settles: Option<usize>,
}

/// A reservation in the ledger: its line number, for `settle`, and what it holds.
#[derive(Debug)]
pub struct Hold {
    line: usize,
    tokens: u64,
    cost: f64,
}

/// What `try_reserve` found.
#[derive(Debug)]
pub enum Reserve {
    /// Held: the request may ask for at most this much output.
    Granted(Hold, u64),
    /// Other running members' holds take the room; it frees as their requests complete.
    Busy,
}

/// This process's place in its family.
#[derive(Debug, Clone)]
pub struct Family {
    dir: PathBuf,
    pub id: String,
    pub me: String,
    pub name: String,
    pub parent: Option<String>,
    /// The parent's pid, watched so this member stops when the parent does.
    parent_pid: Option<u32>,
    budget_tokens: u64,
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

/// The same shape as a session id: a spawned child saves its session under its member id.
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

/// Owner-only, through a temporary file and a rename, so a reader never sees half of it.
fn write_whole(path: &Path, body: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
    crate::config::restrict_to_owner(&tmp)?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))
}

fn store(dir: &Path, registry: &Registry) -> Result<()> {
    write_whole(
        &dir.join("registry.json"),
        &serde_json::to_vec_pretty(registry)?,
    )
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

fn answer_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.answer"))
}

fn log_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.log"))
}

fn stop_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.stop"))
}

/// Why a member stops on its own.
pub enum Stop {
    /// The agent that spawned it exited.
    Parent,
    /// `shutdown`, or a crossed budget, asked it to.
    Asked,
}

fn ledger(dir: &Path) -> Result<Vec<Spend>> {
    let path = dir.join("ledger.jsonl");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("parsing {}", path.display())))
        .collect()
}

/// Spent and held tokens and USD. Settled lines count as spent, and so do the open holds of
/// members no longer running: such a request spent an unknown amount, at most its hold. Open
/// holds of running members are held: they settle when their requests complete.
fn totals(lines: &[Spend], running: &[String]) -> (u64, f64, u64, f64) {
    let settled: Vec<usize> = lines.iter().filter_map(|s| s.settles).collect();
    let mut t = (0u64, 0.0, 0u64, 0.0);
    for (i, s) in lines.iter().enumerate() {
        if s.hold && settled.contains(&i) {
            continue;
        }
        if s.hold && running.contains(&s.member) {
            t.2 = t.2.saturating_add(s.tokens);
            t.3 += s.cost;
        } else {
            t.0 = t.0.saturating_add(s.tokens);
            t.1 += s.cost;
        }
    }
    t
}

/// `budget` less its reserve.
fn below_reserve(budget: u64) -> u64 {
    budget / 100 * (100 - RESERVE_PERCENT) + budget % 100 * (100 - RESERVE_PERCENT) / 100
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
/// Compared with this executable's name, since `spawn` starts every member from it.
fn is_minima(pid: u32) -> bool {
    // Not `comm`, which Linux cuts to 15 bytes. A replaced binary reads as "PATH (deleted)".
    #[cfg(target_os = "linux")]
    let name = std::fs::read_link(format!("/proc/{pid}/exe"))
        .map(|p| {
            p.to_string_lossy()
                .trim_end_matches(" (deleted)")
                .to_string()
        })
        .unwrap_or_default();
    #[cfg(not(target_os = "linux"))]
    let name = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let exe = std::env::current_exe().ok();
    let own = exe.as_deref().and_then(Path::file_name);
    let theirs = Path::new(name.trim()).file_name();
    own.is_some() && theirs == own
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
            let (budget_tokens, budget_cost, parent_pid) = update(&dir, |registry| {
                if let Some(m) = registry.members.iter_mut().find(|m| m.id == child.me) {
                    m.pid = pid;
                    m.ended_at = None;
                }
                let Some(parent) = registry.members.iter().find(|m| m.id == child.parent) else {
                    bail!("parent {} is not in the family", child.parent);
                };
                Ok((registry.budget_tokens, registry.budget_cost, parent.pid))
            })?;
            Family {
                dir,
                id: child.family,
                me: child.me,
                name: child.name,
                parent: Some(child.parent),
                parent_pid: Some(parent_pid),
                budget_tokens,
                budget_cost,
            }
        }
        None => {
            // Required: the budget is the one bound on how much a family can spend.
            let Some(budget_tokens) = cli.budget_tokens else {
                bail!("-r needs --budget-tokens N, the most tokens the whole family may use");
            };
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
                budget_tokens,
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
                parent_pid: None,
                budget_tokens,
                budget_cost: cli.budget_cost,
            }
        }
    };
    if bounds.sandbox {
        bounds.writable.push(std::fs::canonicalize(&family.dir)?);
        // A child started by `spawn` under its parent's sandboxed `bash` already has the policy.
        bounds.confined = family.parent.is_some() && sandboxed();
    }
    // SAFETY: called from `start` before the runtime or any other thread exists.
    unsafe {
        std::env::set_var(ENV_FAMILY, &family.id);
        std::env::set_var(ENV_SELF, &family.me);
    }
    Ok(family)
}

/// Whether this process runs under a Seatbelt profile. Landlock stacks a nested policy, so Linux
/// always applies it again.
#[cfg(target_os = "macos")]
fn sandboxed() -> bool {
    unsafe extern "C" {
        fn sandbox_check(
            pid: libc::pid_t,
            operation: *const libc::c_char,
            kind: libc::c_int,
            ...
        ) -> libc::c_int;
    }
    // SAFETY: a null operation with SANDBOX_FILTER_NONE (0) reads no further arguments.
    unsafe { sandbox_check(libc::getpid(), std::ptr::null(), 0) == 1 }
}

#[cfg(not(target_os = "macos"))]
fn sandboxed() -> bool {
    false
}

/// The least output a request is granted when the budget is short. Below it, the request waits
/// for other members' holds, or fails when the budget itself is spent.
const MIN_GRANT: u64 = 4096;

/// The family's budget as the agent loop holds requests against it. One request runs at a time,
/// so at most one hold is open.
pub struct Ledger {
    family: Family,
    hold: std::cell::RefCell<Option<Hold>>,
}

impl Ledger {
    pub fn new(family: Family) -> Self {
        Self {
            family,
            hold: std::cell::RefCell::new(None),
        }
    }
}

impl minima_core::agent::Budget for Ledger {
    fn prompt(&self) -> String {
        self.family.prompt()
    }

    /// Waits while other members' holds take the room. Crossing the budget stops this member's
    /// descendants; this one reports why.
    fn reserve<'a>(
        &'a self,
        input: u64,
        ceiling: u64,
        price: &'a dyn Fn(u64, u64) -> f64,
        cancel: &'a minima_core::Cancel,
    ) -> futures_util::future::LocalBoxFuture<'a, Result<Option<u64>>> {
        Box::pin(async move {
            loop {
                match self.family.try_reserve(input, ceiling, MIN_GRANT, price) {
                    Ok(Reserve::Granted(hold, output)) => {
                        *self.hold.borrow_mut() = Some(hold);
                        return Ok(Some(output));
                    }
                    // Holds settle as their requests complete, within one response.
                    Ok(Reserve::Busy) => tokio::select! {
                        () = tokio::time::sleep(Duration::from_millis(250)) => {}
                        () = cancel.cancelled() => return Ok(None),
                    },
                    Err(e) => {
                        // Off the runtime's workers: `shutdown` sleeps through the grace period.
                        let family = self.family.clone();
                        let stopped =
                            tokio::task::spawn_blocking(move || family.stop_descendants())
                                .await
                                .map_err(anyhow::Error::from)
                                .and_then(|r| r);
                        return match stopped {
                            Err(stop) => {
                                Err(e.context(format!("shutting the family down failed: {stop:#}")))
                            }
                            Ok(()) => Err(e),
                        };
                    }
                }
            }
        })
    }

    fn settle(&self, used: Option<(u64, f64)>) -> Result<()> {
        let Some(hold) = self.hold.borrow_mut().take() else {
            return Ok(());
        };
        match used {
            Some((tokens, cost)) => self.family.settle(hold, tokens, cost),
            None => self.family.forfeit(hold),
        }
    }
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

    /// Resolves once `shutdown` asks this member to stop, or its parent has exited.
    pub async fn stopped(&self) -> Stop {
        let marker = stop_path(&self.dir, &self.me);
        loop {
            if marker.exists() {
                return Stop::Asked;
            }
            // Sampled each second: a pid is reused only once the counter wraps, which takes longer.
            if self.parent_pid.is_some_and(|pid| !alive(pid)) {
                return Stop::Parent;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// A spawned child's final answer, for its parent's `wait`.
    pub fn answer(&self, text: &str) -> Result<()> {
        if self.parent.is_none() {
            return Ok(());
        }
        write_whole(&answer_path(&self.dir, &self.me), text.as_bytes())
    }

    /// Holds room for a request of `input` tokens and at most `output` of output, granting less
    /// output when the budget has less left, but no less than `floor`. `price(input, output)` is
    /// the request's cost. Busy when only running members' holds stand in the way; fails when
    /// spending alone leaves too little. The check and the hold share one lock, so two members
    /// cannot both take the same remainder.
    pub fn try_reserve(
        &self,
        input: u64,
        output: u64,
        floor: u64,
        price: &dyn Fn(u64, u64) -> f64,
    ) -> Result<Reserve> {
        let _lock = lock(&self.dir)?;
        let lines = ledger(&self.dir)?;
        let running: Vec<String> = load(&self.dir)?
            .members
            .into_iter()
            .filter(Member::running)
            .map(|m| m.id)
            .collect();
        let (spent, spent_cost, held, held_cost) = totals(&lines, &running);
        let floor = floor.min(output).max(1);
        let token_line = below_reserve(self.budget_tokens);
        let cost_line = self
            .budget_cost
            .map(|b| b * (100 - RESERVE_PERCENT) as f64 / 100.0);
        // The most output that fits beside `used` tokens and `used_cost` USD.
        let by_tokens = |used: u64| token_line.saturating_sub(used.saturating_add(input));
        let fits = |used: u64, used_cost: f64| -> u64 {
            let mut most = output.min(by_tokens(used));
            if let Some(line) = cost_line {
                let left = line - used_cost - price(input, 0);
                let per_token = price(0, 1_000_000) / 1e6;
                most = most.min(match left {
                    l if l < 0.0 => 0,
                    l if per_token > 0.0 => (l / per_token) as u64,
                    _ => u64::MAX,
                });
            }
            most
        };
        let grant = fits(spent.saturating_add(held), spent_cost + held_cost);
        if grant >= floor {
            let hold = Hold {
                line: lines.len(),
                tokens: input.saturating_add(grant),
                cost: price(input, grant),
            };
            self.record(hold.tokens, hold.cost, true, None)?;
            return Ok(Reserve::Granted(hold, grant));
        }
        if fits(spent, spent_cost) >= floor {
            return Ok(Reserve::Busy);
        }
        let budget = self.budget_tokens;
        if by_tokens(spent) < floor {
            bail!(
                "family token budget: {spent} of {budget} spent, and the next request needs at \
                 least {} more; this agent stops, with its descendants",
                input.saturating_add(floor)
            );
        }
        bail!(
            "family cost budget: ${spent_cost:.4} of ${:.4} spent, and the next request needs at \
             least ${:.4} more; this agent stops, with its descendants",
            self.budget_cost.unwrap_or_default(),
            price(input, floor)
        );
    }

    /// Records a request's usage in place of its hold.
    pub fn settle(&self, hold: Hold, tokens: u64, cost: f64) -> Result<()> {
        let _lock = lock(&self.dir)?;
        self.record(tokens, cost, false, Some(hold.line))
    }

    /// Settles a hold whose request did not complete at its full size: what it spent is unknown,
    /// and at most the hold. Without this, the member's own hold would keep it `Busy`.
    pub fn forfeit(&self, hold: Hold) -> Result<()> {
        let (tokens, cost) = (hold.tokens, hold.cost);
        self.settle(hold, tokens, cost)
    }

    /// Appends one ledger line. The caller holds the lock.
    fn record(&self, tokens: u64, cost: f64, hold: bool, settles: Option<usize>) -> Result<()> {
        append(
            &self.dir.join("ledger.jsonl"),
            &Spend {
                member: self.me.clone(),
                at: now(),
                tokens,
                cost,
                hold,
                settles,
            },
        )
    }

    /// Stops this member's subtree after a crossed budget: the descendants by signal, this one
    /// by refusing each request. Its parent learns why from its answer.
    pub fn stop_descendants(&self) -> Result<()> {
        shutdown(&self.dir, Duration::from_secs(5), &Scope::Below(&self.me)).map(|_| ())
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
             task and prints its id. It runs once, headless, with its own context.\n\
             - `{exe} -r wait NAME_OR_ID` prints the child's final answer, waiting up to 100 \
             seconds for it; run it again while the child is still working. `--timeout SECS` \
             waits longer; keep it under the bash timeout.\n\
             - `{exe} -r agents` lists the family.\n\
             - `{exe} -r shutdown` stops your children and theirs.\n\
             Your children stop when you exit, so wait for every answer you need before giving \
             your own. Children share your root, sandbox and budget.",
            name = self.name,
            me = self.me,
        )
    }
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

/// `target`'s final answer, once it has one. Fails when it exits without one, or is still
/// running at `timeout`.
fn wait(dir: &Path, target: &str, timeout: Duration) -> Result<String> {
    let deadline = Instant::now() + timeout;
    loop {
        let member = resolve(&load(dir)?, target)?.clone();
        // Sampled before the answer is read: a child writes its answer before it exits.
        let running = member.running();
        match std::fs::read_to_string(answer_path(dir, &member.id)) {
            Ok(answer) => return Ok(answer),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("reading the answer"),
        }
        if !running {
            bail!(
                "{} ({}) exited without an answer; its output is in {}",
                member.name,
                member.id,
                log_path(dir, &member.id).display()
            );
        }
        if Instant::now() >= deadline {
            bail!(
                "{} ({}) is still running after {}s; run wait again",
                member.name,
                member.id,
                timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Which members `shutdown` stops.
enum Scope<'a> {
    /// Every member: `shutdown` run outside the family.
    All,
    /// This member's descendants: `shutdown` run by an agent, or a budget it crossed.
    Below(&'a str),
}

fn targets(registry: &Registry, scope: &Scope) -> Vec<Member> {
    let mut below: Vec<&str> = match scope {
        Scope::Below(id) => vec![id],
        _ => Vec::new(),
    };
    // A parent is listed before its children, so one pass in order reaches every descendant.
    for m in &registry.members {
        if let Some(p) = &m.parent
            && below.contains(&p.as_str())
        {
            below.push(&m.id);
        }
    }
    registry
        .members
        .iter()
        .filter(|m| m.running())
        .filter(|m| match scope {
            Scope::All => true,
            Scope::Below(id) => m.id != *id && below.contains(&m.id.as_str()),
        })
        .cloned()
        .collect()
}

/// Asks every member in `scope` to stop through its `.stop` marker, SIGKILLs one still running
/// after `grace`, then prunes. Returns how many were asked. A marker rather than SIGTERM: under
/// the macOS sandbox, a member may signal only processes in its own sandbox.
fn shutdown(dir: &Path, grace: Duration, scope: &Scope) -> Result<usize> {
    let targets = {
        let _lock = lock(dir)?;
        targets(&load(dir)?, scope)
    };
    for m in &targets {
        write_whole(&stop_path(dir, &m.id), b"")?;
    }
    // Not under the lock: a member's exit hook takes it to record its end.
    let deadline = Instant::now() + grace;
    while targets.iter().any(|m| alive(m.pid)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut killed = Vec::new();
    for m in targets.iter().filter(|m| alive(m.pid)) {
        if is_minima(m.pid) {
            signal(m.pid, libc::SIGKILL);
            killed.push(m);
        } else {
            eprintln!(
                "minima: skipped {} ({}): pid {} did not stop, and is not minima or cannot be checked",
                m.name, m.id, m.pid
            );
        }
    }
    if !killed.is_empty() {
        std::thread::sleep(Duration::from_millis(100));
    }
    if let Some(m) = killed.iter().find(|m| alive(m.pid)) {
        bail!(
            "{} ({}) did not stop: pid {} survived SIGKILL",
            m.name,
            m.id,
            m.pid
        );
    }
    let ids: Vec<String> = targets.iter().map(|m| m.id.clone()).collect();
    update(dir, |r| {
        for m in r.members.iter_mut().filter(|m| ids.contains(&m.id)) {
            m.ended_at.get_or_insert_with(now);
        }
        Ok(())
    })?;
    prune(dir)?;
    Ok(targets.len())
}

/// Drops ended members whose parent has ended too, with their answers and logs: while the parent
/// runs, it may still `wait` for the answer. A family with none left is removed whole.
fn prune(dir: &Path) -> Result<usize> {
    let (pruned, empty) = update(dir, |r| {
        let running: Vec<String> = r
            .members
            .iter()
            .filter(|m| m.running())
            .map(|m| m.id.clone())
            .collect();
        let (gone, kept): (Vec<Member>, Vec<Member>) = r.members.drain(..).partition(|m| {
            !running.contains(&m.id) && m.parent.as_ref().is_none_or(|p| !running.contains(p))
        });
        r.members = kept;
        for m in &gone {
            let _ = std::fs::remove_file(answer_path(dir, &m.id));
            let _ = std::fs::remove_file(log_path(dir, &m.id));
            let _ = std::fs::remove_file(stop_path(dir, &m.id));
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
    /// Print a child's final answer, waiting for it.
    Wait {
        #[arg(value_name = "NAME_OR_ID")]
        target: String,
        /// Fail if the child is still running after SECS.
        #[arg(long, value_name = "SECS", default_value_t = 100)]
        timeout: u64,
    },
    /// List the family.
    Agents {
        /// The family to list. Default: this agent's, else the newest for this directory.
        #[arg(long, value_name = "ID")]
        family: Option<String>,
        /// Drop ended members whose parent has ended, with their answers and logs.
        #[arg(long)]
        prune: bool,
    },
    /// Stop agents: ask each to stop, then SIGKILL after the grace period. Run by an agent,
    /// its descendants; otherwise the whole family.
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
        Cmd::Wait { target, timeout } => {
            let (family, _) = own()?;
            let answer = wait(&base.join(family), &target, Duration::from_secs(timeout))?;
            println!("{answer}");
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
            let id = find(&base, family)?;
            // An agent stopping its own family keeps itself and its ancestors.
            let me = own().ok().filter(|(f, _)| *f == id).map(|(_, me)| me);
            let scope = me.as_deref().map_or(Scope::All, Scope::Below);
            let n = shutdown(&base.join(id), Duration::from_secs(grace), &scope)?;
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
    // Printed in the `agents` table, which the model reads.
    if name.is_empty()
        || name.len() > 32
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("--name must be 1 to 32 letters, digits, '-' or '_': {name:?}");
    }
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
        let log_path = log_path(dir, &id);
        let log = std::fs::File::create(&log_path)
            .with_context(|| format!("creating {}", log_path.display()))?;
        crate::config::restrict_to_owner(&log_path)?;
        let mut command = Command::new(&exe);
        // One argument, so a task starting with `-` is not read as a flag.
        command.args(["--recurse", "--no-color", &format!("--prompt={task}")]);
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

    fn ended(mut m: Member) -> Member {
        m.ended_at = Some(1);
        m
    }

    fn registry(dir: &Path, members: Vec<Member>) -> Registry {
        let r = Registry {
            schema: SCHEMA,
            root: dir.into(),
            sandbox: false,
            writable: Vec::new(),
            budget_tokens: 1_000_000,
            budget_cost: None,
            members,
        };
        store(dir, &r).unwrap();
        r
    }

    fn family(dir: &Path, tokens: u64, cost: Option<f64>) -> Family {
        Family {
            dir: dir.into(),
            id: "f".into(),
            me: "a".into(),
            name: ROOT_NAME.into(),
            parent: None,
            parent_pid: None,
            budget_tokens: tokens,
            budget_cost: cost,
        }
    }

    #[test]
    fn wait_returns_the_answer_by_name_or_id() {
        let dir = scratch("answer");
        let me = std::process::id();
        registry(
            &dir,
            vec![
                member("a", "root", None, me),
                member("b", "kid", Some("a"), me),
            ],
        );
        Family {
            me: "b".into(),
            parent: Some("a".into()),
            ..family(&dir, 1, None)
        }
        .answer("hello")
        .unwrap();
        let t = Duration::ZERO;
        assert_eq!(wait(&dir, "kid", t).unwrap(), "hello");
        // Read again, not taken: a second wait sees the same answer.
        assert_eq!(wait(&dir, "b", t).unwrap(), "hello");
    }

    #[test]
    fn wait_tells_a_running_child_from_one_that_exited_without_an_answer() {
        let dir = scratch("no-answer");
        let me = std::process::id();
        registry(
            &dir,
            vec![
                member("a", "root", None, me),
                member("b", "busy", Some("a"), me),
                ended(member("c", "dead", Some("a"), me)),
            ],
        );
        let busy = wait(&dir, "busy", Duration::ZERO).unwrap_err().to_string();
        assert!(busy.contains("still running"), "{busy}");
        let dead = wait(&dir, "dead", Duration::ZERO).unwrap_err().to_string();
        assert!(dead.contains("exited without an answer"), "{dead}");
    }

    #[test]
    fn shutdown_by_an_agent_reaches_its_descendants_only() {
        let dir = scratch("scope");
        let me = std::process::id();
        let r = registry(
            &dir,
            vec![
                member("a", "root", None, me),
                member("b", "kid", Some("a"), me),
                member("c", "grandkid", Some("b"), me),
                member("d", "sibling", Some("a"), me),
            ],
        );
        let ids = |scope: Scope| -> Vec<String> {
            targets(&r, &scope).into_iter().map(|m| m.id).collect()
        };
        assert_eq!(ids(Scope::Below("b")), ["c"]);
        assert_eq!(ids(Scope::Below("a")), ["b", "c", "d"]);
        assert_eq!(ids(Scope::All), ["a", "b", "c", "d"]);
    }

    fn free(_: u64, _: u64) -> f64 {
        0.0
    }

    /// Members `a` and `b` running, `gone` ended, under a budget of `tokens` and `cost`.
    fn budgeted(name: &str, tokens: u64, cost: Option<f64>) -> (PathBuf, Family, Family) {
        let dir = scratch(name);
        let me = std::process::id();
        registry(
            &dir,
            vec![
                member("a", "root", None, me),
                member("b", "kid", Some("a"), me),
                ended(member("gone", "old", Some("a"), me)),
            ],
        );
        let a = family(&dir, tokens, cost);
        let b = Family {
            me: "b".into(),
            ..a.clone()
        };
        (dir, a, b)
    }

    fn granted(r: Result<Reserve>) -> (Hold, u64) {
        match r.unwrap() {
            Reserve::Granted(hold, output) => (hold, output),
            Reserve::Busy => panic!("busy"),
        }
    }

    #[test]
    fn the_budget_holds_back_its_reserve_and_counts_every_member() {
        let (_dir, a, b) = budgeted("budget", 1000, None);
        let (hold, _) = granted(a.try_reserve(500, 100, 100, &free));
        a.settle(hold, 600, 0.0).unwrap();
        let (hold, _) = granted(b.try_reserve(100, 100, 100, &free));
        b.settle(hold, 200, 0.0).unwrap();
        // 950 is the line: 1000 less 5%, and 800 is spent.
        assert!(a.try_reserve(51, 100, 100, &free).is_err());
        granted(a.try_reserve(50, 100, 100, &free));
    }

    #[test]
    fn a_short_budget_grants_less_output_down_to_the_floor() {
        let (_dir, a, _) = budgeted("grant", 1000, None);
        let (_, output) = granted(a.try_reserve(100, 2000, 50, &free));
        assert_eq!(output, 850);
    }

    #[test]
    fn a_running_members_hold_makes_another_wait_until_it_settles() {
        let (_dir, a, b) = budgeted("busy", 1000, None);
        let (hold, _) = granted(a.try_reserve(800, 100, 100, &free));
        assert!(matches!(
            b.try_reserve(0, 100, 100, &free),
            Ok(Reserve::Busy)
        ));
        a.settle(hold, 10, 0.0).unwrap();
        granted(b.try_reserve(0, 100, 100, &free));
    }

    #[test]
    fn an_ended_members_open_hold_counts_as_spent() {
        let (_dir, a, b) = budgeted("ended-hold", 1000, None);
        let gone = Family {
            me: "gone".into(),
            ..a.clone()
        };
        granted(gone.try_reserve(800, 100, 100, &free));
        // Not busy: an ended member's hold never settles.
        let err = b.try_reserve(0, 100, 100, &free).unwrap_err().to_string();
        assert!(err.contains("family token budget"), "{err}");
    }

    #[test]
    fn a_forfeited_hold_counts_whole_and_no_longer_waits() {
        let (_dir, a, b) = budgeted("forfeit", 1000, None);
        let (hold, _) = granted(a.try_reserve(800, 100, 100, &free));
        a.forfeit(hold).unwrap();
        assert!(b.try_reserve(0, 100, 100, &free).is_err());
        assert!(a.try_reserve(0, 100, 100, &free).is_err());
    }

    #[test]
    fn concurrent_members_cannot_both_take_one_remainder() {
        let (_dir, a, _) = budgeted("race", 1000, None);
        // Room for 9 holds of 100 under the 950 line, and 16 members asking at once.
        let passed = std::thread::scope(|s| {
            let handles: Vec<_> = (0..16)
                .map(|i| {
                    let f = Family {
                        me: format!("m{i}"),
                        ..a.clone()
                    };
                    s.spawn(move || {
                        matches!(f.try_reserve(0, 100, 100, &free), Ok(Reserve::Granted(..)))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|&ok| ok)
                .count()
        });
        assert_eq!(passed, 9);
    }

    #[test]
    fn a_huge_token_budget_does_not_overflow() {
        let (_dir, a, _) = budgeted("huge", u64::MAX, None);
        assert!(a.try_reserve(u64::MAX, 1, 1, &free).is_err());
        granted(a.try_reserve(u64::MAX / 2, 1, 1, &free));
    }

    #[test]
    fn a_cost_budget_grants_less_output_and_is_checked_apart_from_tokens() {
        let (_dir, a, _) = budgeted("cost", 1_000_000, Some(1.0));
        let price = |input: u64, output: u64| (input + output) as f64 * 0.001;
        let (hold, _) = granted(a.try_reserve(10, 890, 1, &price));
        a.settle(hold, 900, 0.9).unwrap();
        // $0.95 is the line and $0.90 spent: $0.04 is left after the input's $0.01.
        let err = a.try_reserve(10, 100, 50, &price).unwrap_err().to_string();
        assert!(err.contains("cost budget"), "{err}");
        let (_, output) = granted(a.try_reserve(10, 100, 30, &price));
        assert!((39..=40).contains(&output), "{output}");
    }

    #[test]
    fn spawn_refuses_a_name_that_could_forge_a_table_row() {
        let dir = scratch("name");
        for name in ["", "a\nroot  self", "has space", &"x".repeat(33)] {
            let err = spawn(&dir, "a", name, None, None, None, "t")
                .unwrap_err()
                .to_string();
            assert!(err.contains("--name"), "{name:?}: {err}");
        }
    }

    #[test]
    fn is_minima_matches_this_executable_not_another() {
        assert!(is_minima(std::process::id()));
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        assert!(!is_minima(sleeper.id()));
        sleeper.kill().unwrap();
        let _ = sleeper.wait();
    }

    #[test]
    fn shutdown_skips_a_pid_that_is_not_minima() {
        let dir = scratch("recycled");
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        registry(&dir, vec![member("b", "kid", Some("a"), sleeper.id())]);
        shutdown(&dir, Duration::from_millis(100), &Scope::All).unwrap();
        assert!(sleeper.try_wait().unwrap().is_none(), "sleep was signalled");
        sleeper.kill().unwrap();
        let _ = sleeper.wait();
    }

    #[test]
    fn prune_keeps_an_answer_while_the_parent_runs() {
        let dir = scratch("prune");
        let me = std::process::id();
        registry(
            &dir,
            vec![
                member("a", "root", None, me),
                ended(member("b", "kid", Some("a"), me)),
            ],
        );
        std::fs::write(answer_path(&dir, "b"), "done").unwrap();
        assert_eq!(prune(&dir).unwrap(), 0);
        assert!(answer_path(&dir, "b").exists());
        update(&dir, |r| {
            r.members[0].ended_at = Some(2);
            Ok(())
        })
        .unwrap();
        assert_eq!(prune(&dir).unwrap(), 2);
        assert!(!dir.exists());
    }

    #[test]
    fn files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        registry(&dir, vec![member("a", "root", None, std::process::id())]);
        let f = Family {
            me: "b".into(),
            parent: Some("a".into()),
            ..family(&dir, 1000, None)
        };
        f.answer("hi").unwrap();
        f.try_reserve(1, 1, 1, &free).unwrap();
        for f in ["registry.json", "b.answer", "ledger.jsonl"] {
            let mode = std::fs::metadata(dir.join(f)).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{f}");
        }
    }
}
