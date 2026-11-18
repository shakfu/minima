//! minima: a minimal agent harness.

// Process groups, owner-only files and the exit signals are Unix. A non-Unix build would run
// without them and break what the README promises, so it does not build.
#[cfg(not(unix))]
compile_error!("minima supports Unix only (Linux and macOS)");

mod agent;
mod cache;
mod cancel;
mod config;
mod family;
mod frontend;
mod herdr;
mod price;
mod prompt;
mod provider;
mod session;
mod state;
mod term;
mod theme;
mod tools;
mod turn;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::agent::Agent;
use crate::cancel::Cancel;
use crate::config::{Bounds, Cli};
use crate::family::Family;
use crate::frontend::Frontend;
use crate::frontend::headless::Headless;
use crate::frontend::json::Json;
use crate::provider::{Provider, Role, http::Http, mock::Mock};
use crate::session::Session;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    term::install_panic_hook();
    // Dropped on return and on unwind, which release builds keep.
    let _jobs = KillBackgroundJobs;

    match start() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("minima: {}", frontend::printable(&format!("{e:#}")));
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<ExitCode> {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(code) = family::dispatch(&args)? {
        return Ok(code);
    }
    let mut cli = Cli::parse_from(args);
    #[cfg(feature = "codemode")]
    tools::js::init_variant()?;
    let inherited = match cli.recurse {
        true => family::inherit(&mut cli)?,
        false => None,
    };
    let root = cli.resolve_root()?;
    std::env::set_current_dir(&root)
        .with_context(|| format!("changing to root {}", root.display()))?;
    theme::init(cli.no_color);
    let mut bounds = cli.bounds(root)?;
    // Before the runtime: `join` sets the environment every `bash` command inherits, which is
    // sound only while no other thread runs.
    let family = match cli.recurse {
        true => Some(family::join(inherited, &mut bounds, &cli)?),
        false => None,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    exit_on_hangup_or_terminate(&runtime, family.clone())?;

    // The kernel policy can fail to install, so it is tried once here. It runs against the real
    // bounds, so a `--writable` path the policy will not take fails here rather than on the
    // model's first command.
    if bounds.sandbox {
        runtime.block_on(tools::preflight(&bounds))?;
    }

    // Loaded before config, which takes the session's provider and model where no flag names them.
    let config_dir = config::config_dir();
    let resumed = match (&cli.resume, cli.continue_last) {
        (None, false) => None,
        (id, _) => {
            let dir = config_dir
                .as_deref()
                .context("no config directory to resume from: set $HOME or $XDG_CONFIG_HOME")?;
            Some(match id {
                Some(id) => Session::open(dir, &bounds.root, id)?,
                None => Session::newest(dir, &bounds.root)?,
            })
        }
    };
    if let Some((_, saved)) = &resumed {
        cli.adopt(&saved.provider, &saved.model);
    }

    let mut config = runtime.block_on(cli.resolve())?;
    // Not in resolve(), which stays free of network calls the provider does not need.
    if cli.mock.is_none()
        && let Some(listing) = runtime.block_on(price::lookup(&config, cli.refresh_models))
    {
        listing.apply(&mut config);
    }
    let provider = match &cli.mock {
        Some(path) => Provider::Mock(Mock::load(path)?),
        None => Provider::Http(Http::new()?),
    };
    if let Some(family) = &family {
        family.describe(&config.provider, &config.model)?;
    }
    let mut agent = Agent::with_bounds(provider, config, bounds.clone());
    if let Some(family) = family.clone() {
        agent.join(family);
    }
    // Recorded once a turn streams, so a model the provider rejects is never remembered. Not in
    // resolve(), so config resolution has no disk side effect and its tests write nothing.
    if cli.mock.is_none() {
        agent
            .on_first_turn(|config| state::State::load().remember(&config.provider, &config.model));
    }

    let session = match resumed {
        Some((session, saved)) => {
            agent.resume(saved.restore(agent.provider(), agent.model()));
            Some(session)
        }
        None => config_dir.map(|dir| Session::new(&dir, &bounds.root)),
    };
    // `--no-session` with `--continue` reads the session and writes nothing back.
    let session = session.filter(|_| !cli.no_session);

    let code = match cli.prompt.as_deref() {
        Some(prompt) => runtime.block_on(headless(&mut agent, session, prompt, cli.json, &bounds)),
        None => frontend::repl::run(&runtime, &mut agent, session, &bounds, cli.first_prompt())
            .map(|()| ExitCode::SUCCESS),
    };
    if let Some(family) = &family {
        family.leave();
    }
    code
}

/// Background jobs from `bash` live until minima exits, not past it.
struct KillBackgroundJobs;

impl Drop for KillBackgroundJobs {
    fn drop(&mut self) {
        tools::kill_background();
    }
}

/// `process::exit` runs no destructors, so this kills the jobs and restores the terminal itself.
/// Without it, closing the terminal would leave the jobs running: they are in their own process
/// groups, which the hangup does not reach.
fn exit_on_hangup_or_terminate(
    runtime: &tokio::runtime::Runtime,
    family: Option<Family>,
) -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    // Registered here rather than in the task, so a signal that arrives before the task first runs
    // is not lost.
    let _context = runtime.enter();
    let mut hangup = signal(SignalKind::hangup())?;
    let mut terminate = signal(SignalKind::terminate())?;
    runtime.spawn(async move {
        // 128 + the signal number, as a shell reports it.
        let code = tokio::select! {
            _ = hangup.recv() => 129,
            _ = terminate.recv() => 143,
        };
        term::restore();
        tools::kill_background();
        if let Some(family) = &family {
            family.leave();
        }
        std::process::exit(code);
    });
    Ok(())
}

/// Ctrl-C latches the same flag Esc does in the REPL, so the loop samples one thing either way.
async fn headless(
    agent: &mut Agent,
    mut session: Option<Session>,
    prompt: &str,
    as_json: bool,
    bounds: &Bounds,
) -> Result<ExitCode> {
    let cancel = Cancel::new();
    let watcher = tokio::spawn({
        let cancel = cancel.clone();
        let family = agent.family().cloned();
        async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
                // tokio's handler stays installed, so without this a run the cancel did not end
                // could only be killed from another terminal.
                if tokio::signal::ctrl_c().await.is_ok() {
                    tools::kill_background();
                    if let Some(family) = &family {
                        family.leave();
                    }
                    std::process::exit(130);
                }
            }
        }
    });

    let mut plain = Headless::default();
    let mut json = Json::new(std::io::stdout(), bounds);
    json.estimate = agent.cost_is_estimate();
    let frontend: &mut dyn Frontend = if as_json { &mut json } else { &mut plain };
    let result = agent.run(prompt, frontend, &cancel).await;
    watcher.abort();
    if let Some(family) = agent.family() {
        let answer = match &result {
            Err(e) => format!("failed: {e:#}"),
            Ok(()) if cancel.is_cancelled() => "cancelled".into(),
            Ok(()) => last_answer(agent.messages()),
        };
        if let Err(e) = family.answer_parent(&answer) {
            eprintln!(
                "minima: answer not sent to the parent: {}",
                frontend::printable(&format!("{e:#}"))
            );
        }
    }
    // Reported, not fatal: the answer is already printed.
    if let Some(s) = &mut session
        && let Err(e) = s.save(agent.provider(), agent.model(), agent.messages())
    {
        eprintln!(
            "minima: session not saved: {}",
            frontend::printable(&format!("{e:#}"))
        );
    }
    if as_json {
        json.result(
            &result,
            cancel.is_cancelled(),
            session.filter(|s| s.saved).map(|s| s.id),
        );
    } else {
        println!();
    }
    // 128 + SIGINT, so a script can tell a cancelled run from a finished one.
    if cancel.is_cancelled() {
        return Ok(ExitCode::from(130));
    }
    match result {
        // The result record already carries the error; stderr would repeat it.
        Err(_) if as_json => Ok(ExitCode::FAILURE),
        result => result.map(|()| ExitCode::SUCCESS),
    }
}

/// A spawned agent's final answer: its last text.
fn last_answer(messages: &[provider::Message]) -> String {
    messages
        .iter()
        .rev()
        .filter(|m| m.role == Role::Assistant)
        .find_map(|m| m.content.clone().filter(|c| !c.trim().is_empty()))
        .unwrap_or_else(|| "(no answer)".into())
}
