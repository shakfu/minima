//! minima: a minimal agent harness. See README.md before adding anything.

mod agent;
mod cache;
mod cancel;
mod config;
mod frontend;
mod provider;
mod state;
mod term;
mod theme;
mod tools;
mod turn;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::agent::Agent;
use crate::cancel::Cancel;
use crate::config::Cli;
use crate::frontend::headless::Headless;
use crate::provider::{Provider, http::Http, mock::Mock};

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    term::install_panic_hook();

    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("minima: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<()> {
    let cli = Cli::parse();
    theme::init(cli.no_color);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let config = runtime.block_on(cli.resolve())?;
    // Remembered here rather than inside resolve(), so config resolution has no disk side effect
    // and its tests write nothing.
    if cli.mock.is_none() {
        state::State::load().remember(&config.provider, &config.model);
    }
    let provider = match &cli.mock {
        Some(path) => Provider::Mock(Mock::load(path)?),
        None => Provider::Http(Http::new()?),
    };
    let mut agent = Agent::new(provider, config);

    match cli.prompt.as_deref() {
        Some(prompt) => runtime.block_on(headless(&mut agent, prompt)),
        None => frontend::repl::run(&runtime, &mut agent),
    }
}

/// Ctrl-C latches the same flag Esc does in the REPL, so the loop samples one thing either way.
async fn headless(agent: &mut Agent, prompt: &str) -> Result<()> {
    let cancel = Cancel::new();
    let watcher = tokio::spawn({
        let cancel = cancel.clone();
        async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
            }
        }
    });

    let mut frontend = Headless::new(false);
    let result = agent.run(prompt, &mut frontend, &cancel).await;
    watcher.abort();
    println!();
    result
}
