//! minima: a minimal agent harness.

mod agent;
mod cache;
mod cancel;
mod config;
mod frontend;
mod price;
mod prompt;
mod provider;
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
use crate::frontend::Frontend;
use crate::frontend::headless::Headless;
use crate::frontend::json::Json;
use crate::provider::{Provider, http::Http, mock::Mock};

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
    let cli = Cli::parse();
    let root = cli.resolve_root()?;
    std::env::set_current_dir(&root)
        .with_context(|| format!("changing to root {}", root.display()))?;
    theme::init(cli.no_color);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    #[cfg(unix)]
    exit_on_hangup_or_terminate(&runtime)?;

    let bounds = cli.bounds(root)?;
    // The kernel policy can fail to install, so it is tried once here. It runs against the real
    // bounds, so a `--writable` path the policy will not take fails here rather than on the
    // model's first command.
    if bounds.sandbox {
        runtime.block_on(tools::preflight(&bounds))?;
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
    let mut agent = Agent::with_bounds(provider, config, bounds.clone());
    // Recorded once a turn streams, so a model the provider rejects is never remembered. Not in
    // resolve(), so config resolution has no disk side effect and its tests write nothing.
    if cli.mock.is_none() {
        agent
            .on_first_turn(|config| state::State::load().remember(&config.provider, &config.model));
    }

    match cli.prompt.as_deref() {
        Some(prompt) => runtime.block_on(headless(&mut agent, prompt, cli.json, &bounds)),
        None => frontend::repl::run(&runtime, &mut agent).map(|()| ExitCode::SUCCESS),
    }
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
#[cfg(unix)]
fn exit_on_hangup_or_terminate(runtime: &tokio::runtime::Runtime) -> Result<()> {
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
        std::process::exit(code);
    });
    Ok(())
}

/// Ctrl-C latches the same flag Esc does in the REPL, so the loop samples one thing either way.
async fn headless(
    agent: &mut Agent,
    prompt: &str,
    as_json: bool,
    bounds: &Bounds,
) -> Result<ExitCode> {
    let cancel = Cancel::new();
    let watcher = tokio::spawn({
        let cancel = cancel.clone();
        async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
            }
        }
    });

    let mut plain = Headless::default();
    let mut json = Json::new(std::io::stdout(), bounds);
    json.estimate = agent.cost_is_estimate();
    let frontend: &mut dyn Frontend = if as_json { &mut json } else { &mut plain };
    let result = agent.run(prompt, frontend, &cancel).await;
    watcher.abort();
    if as_json {
        json.result(&result, cancel.is_cancelled());
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
