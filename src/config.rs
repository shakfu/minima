//! Flags, then environment, then the provider registry, then the model cache. No config format.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;

use minima_core::ResolveError;
use minima_core::provider::Dialect;

pub use minima_core::config::{Config, TOOL_OUTPUT_CAP};
pub use minima_core::fs::restrict_to_owner;

/// What one tool call may reach. Resolved once at startup, so no tool re-reads the flags.
///
/// With `sandbox` off nothing is bounded. On, the kernel policy confines `bash` and everything it
/// starts, and `write` and `edit` are held to the root in minima's own process, where the kernel
/// policy does not reach. The two sets differ on purpose: `bash` may also write `$TMPDIR`,
/// `/dev/null` and the ecosystem caches, which no build works without, and the file tools need none
/// of them.
#[derive(Debug, Clone)]
pub struct Bounds {
    pub sandbox: bool,
    /// Canonical, because `confine_path` compares against it and both sandbox backends match on
    /// the resolved path.
    pub root: PathBuf,
    /// `--sandbox` only, and `bash` only. The file tools never leave the root.
    pub writable: Vec<PathBuf>,
    /// A family member already inside its root's Seatbelt sandbox, which refuses a nested one.
    /// `bash` runs in the inherited sandbox, whose policy is the family's.
    pub confined: bool,
}

impl Bounds {
    /// Tests only. A run resolves its bounds once, through `Cli::bounds`.
    #[cfg(test)]
    pub fn new(sandbox: bool, root: PathBuf) -> Self {
        Self {
            sandbox,
            root,
            writable: Vec::new(),
            confined: false,
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(name = "minima", version, about, max_term_width = 80)]
pub struct Cli {
    /// Directory the agent works in, and the bound --sandbox applies. Default: the
    /// current directory.
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Confine writes to the root: bash under the platform's filesystem sandbox, write
    /// and edit by path.
    #[arg(long)]
    pub sandbox: bool,

    /// Another directory bash may write to under --sandbox. Repeatable.
    #[arg(long, value_name = "DIR")]
    pub writable: Vec<PathBuf>,

    /// Answer one prompt, print the result, exit.
    #[arg(short = 'p', long, value_name = "TEXT")]
    pub prompt: Option<String>,

    /// Start the REPL and submit this as the first prompt, as if typed. Words are
    /// joined with spaces.
    #[arg(value_name = "PROMPT", conflicts_with = "prompt")]
    pub first: Vec<String>,

    /// With -p: print JSON lines, ending in a `result` record.
    // Conflicts with the positional prompt too: clap waives `requires` for an arg that conflicts
    // with one given, so `--json words` would otherwise start the REPL and drop --json.
    #[arg(long, requires = "prompt", conflicts_with = "first")]
    pub json: bool,

    /// Serve the Agent Client Protocol on stdin and stdout, for an editor or another
    /// client. Each session asks before any tool but read runs.
    #[arg(
        long,
        conflicts_with_all = ["prompt", "first", "json", "continue_last", "resume", "recurse"]
    )]
    pub acp: bool,

    /// Provider id. Default: the first one whose key is set.
    #[arg(short = 'P', long, env = "MINIMA_PROVIDER", value_name = "ID")]
    pub provider: Option<String>,

    /// Model id. Default: the last one used with this provider.
    #[arg(short, long, env = "MINIMA_MODEL", value_name = "ID")]
    pub model: Option<String>,

    /// Override the provider's endpoint. The wire format stays the same.
    #[arg(long, env = "MINIMA_BASE_URL", value_name = "URL")]
    pub base_url: Option<String>,

    /// Provider key. Requires --provider.
    #[arg(
        long,
        env = "MINIMA_API_KEY",
        hide_env_values = true,
        value_name = "KEY"
    )]
    pub api_key: Option<String>,

    /// Disable colour. Also off for non-terminals or when NO_COLOR is set.
    #[arg(long)]
    pub no_color: bool,

    /// Context window in tokens. Default: the cached value for the model.
    #[arg(long, env = "MINIMA_CONTEXT", value_name = "N")]
    pub context: Option<u32>,

    /// Replay a scripted JSON stream instead of the network.
    #[arg(long, value_name = "PATH")]
    pub mock: Option<PathBuf>,

    /// Max provider round-trips per user turn.
    #[arg(long, default_value_t = minima_core::resolve::DEFAULT_MAX_TURNS, value_name = "N")]
    pub max_turns: u32,

    /// Re-fetch the model list, ignoring the cache.
    #[arg(long)]
    pub refresh_models: bool,

    /// Resume the newest saved session for the root.
    #[arg(short = 'c', long = "continue", conflicts_with = "resume")]
    pub continue_last: bool,

    /// Resume the saved session with this id. Provider and model come from it unless given.
    #[arg(long, value_name = "ID")]
    pub resume: Option<String>,

    /// Do not save this session.
    #[arg(long, env = "MINIMA_NO_SESSION")]
    pub no_session: bool,

    /// Let the agent spawn child agents and wait for their answers. Needs
    /// --budget-tokens. See docs/dev/recursion.md.
    #[arg(short = 'r', long)]
    pub recurse: bool,

    /// With -r, required: total tokens the family may use. Every agent in it stops
    /// before the next request could cross it.
    #[arg(long, requires = "recurse", value_name = "N")]
    pub budget_tokens: Option<u64>,

    /// With -r: total cost in USD the family may use, as --budget-tokens.
    #[arg(long, requires = "recurse", value_name = "USD")]
    pub budget_cost: Option<f64>,
}

impl Cli {
    /// The positional PROMPT, if any, for the REPL to submit first.
    pub fn first_prompt(&self) -> Option<String> {
        (!self.first.is_empty()).then(|| self.first.join(" "))
    }

    /// Takes the provider and model of a resumed session, where no flag names them.
    pub fn adopt(&mut self, provider: &str, model: &str) {
        let named = self.provider.get_or_insert_with(|| provider.to_string());
        if *named == *provider && self.model.is_none() {
            self.model = Some(model.to_string());
        }
    }

    /// Resolve the root before loading prompts or starting the agent.
    pub fn resolve_root(&self) -> Result<PathBuf> {
        let root = self
            .root
            .clone()
            .unwrap_or(std::env::current_dir().context("getting the current directory")?);
        let root = std::fs::canonicalize(&root)
            .with_context(|| format!("resolving root {}", root.display()))?;
        let metadata =
            std::fs::metadata(&root).with_context(|| format!("reading root {}", root.display()))?;
        if !metadata.is_dir() {
            bail!("root {} is not a directory", root.display());
        }
        Ok(root)
    }

    /// The bounds this run enforces. A `--writable` path that does not resolve is an error rather
    /// than a skip: the user asked for it by name, and dropping it quietly would leave them
    /// believing they granted access that a denied write only reveals mid-turn.
    pub fn bounds(&self, root: PathBuf) -> Result<Bounds> {
        if !self.writable.is_empty() && !self.sandbox {
            bail!("--writable needs --sandbox; without it nothing is bounded");
        }
        let writable = self
            .writable
            .iter()
            .map(|path| {
                std::fs::canonicalize(path)
                    .with_context(|| format!("resolving --writable {}", path.display()))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Bounds {
            sandbox: self.sandbox,
            root,
            writable,
            confined: false,
        })
    }
}

impl Cli {
    /// Resolve into a usable config, consulting the model cache for anything still missing.
    pub async fn resolve(&self) -> Result<Config> {
        if self.mock.is_some() {
            return Ok(Config {
                provider: "mock".into(),
                base_url: self.base_url.clone().unwrap_or_default(),
                api_key: String::new(),
                model: self.model.clone().unwrap_or_else(|| "mock".into()),
                dialect: Dialect::Chat,
                context: self
                    .context
                    .unwrap_or(minima_core::resolve::FALLBACK_CONTEXT),
                context_guessed: false,
                max_output: None,
                adaptive_thinking: false,
                max_turns: self.max_turns,
                pricing: None,
                rates: None,
                cap_output: false,
                cache_dir: None,
            });
        }

        // The pairing is remembered by the caller once a turn streams, not here.
        let state = crate::state::State::load();
        let request = minima_core::Request {
            provider: self.provider.clone(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            context: self.context,
            max_turns: self.max_turns,
            refresh_models: self.refresh_models,
            cache_dir: config_dir(),
            last_provider: state.last_provider().map(String::from),
        };
        request
            .resolve(|var| std::env::var(var).ok(), |p| state.last_model(p))
            .await
            .map_err(flagged)
    }
}

/// The library's error, naming the flag that fixes it.
fn flagged(e: ResolveError) -> anyhow::Error {
    match e {
        ResolveError::KeyWithoutProvider => anyhow::anyhow!(
            "--api-key needs --provider: a key does not say which provider it belongs to"
        ),
        ResolveError::NoProvider => anyhow::anyhow!("{e}, or pass --provider"),
        ResolveError::NoKey { provider, var } => {
            anyhow::anyhow!("no key for {provider}: set ${var} or pass --api-key")
        }
        ResolveError::NoModel { provider, cached } => anyhow::anyhow!(
            "no model for {provider}: pass --model or set MINIMA_MODEL ({cached} cached)"
        ),
        e => e.into(),
    }
}

/// Prompt history. Whatever the user typed goes here verbatim, so it is owner-only.
pub fn history_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("history.txt"))
}

/// `$XDG_CONFIG_HOME/minima`, else `$HOME/.config/minima`. No `dirs` crate for two lines of logic.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(x).join("minima"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("minima"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(model: Option<&str>, context: Option<u32>, refresh: bool) -> Cli {
        Cli {
            root: None,
            sandbox: false,
            writable: Vec::new(),
            prompt: None,
            first: Vec::new(),
            json: false,
            acp: false,
            provider: Some("openrouter".into()),
            model: model.map(String::from),
            base_url: None,
            api_key: Some("k".into()),
            no_color: false,
            context,
            mock: None,
            max_turns: 32,
            refresh_models: refresh,
            continue_last: false,
            resume: None,
            no_session: false,
            recurse: false,
            budget_tokens: None,
            budget_cost: None,
        }
    }

    /// A resumed session supplies the provider and model the flags leave unset. Its model is
    /// kept only for its own provider.
    #[test]
    fn a_session_fills_in_what_the_flags_leave_unset() {
        let pick = |provider: Option<&str>, model: Option<&str>| {
            let mut c = cli(model, None, false);
            c.provider = provider.map(String::from);
            c.adopt("anthropic", "claude-x");
            (c.provider, c.model)
        };
        let some = |s: &str| Some(s.to_string());
        assert_eq!(pick(None, None), (some("anthropic"), some("claude-x")));
        assert_eq!(pick(None, Some("m")), (some("anthropic"), some("m")));
        assert_eq!(
            pick(Some("anthropic"), None),
            (some("anthropic"), some("claude-x"))
        );
        assert_eq!(pick(Some("openai"), None), (some("openai"), None));
    }

    /// Off unless asked for: a run without the flag bounds nothing, as before the flag existed.
    #[test]
    fn the_sandbox_is_off_by_default() {
        assert!(!Cli::try_parse_from(["minima"]).unwrap().sandbox);
        assert!(
            Cli::try_parse_from(["minima", "--sandbox"])
                .unwrap()
                .sandbox
        );
    }

    /// Options parse before and after the words, which join with single spaces.
    #[test]
    fn positional_words_are_the_first_prompt() {
        assert_eq!(
            Cli::try_parse_from(["minima"]).unwrap().first_prompt(),
            None
        );
        let cli =
            Cli::try_parse_from(["minima", "--root", ".", "fix", "the  bug", "--sandbox"]).unwrap();
        assert_eq!(cli.first_prompt().as_deref(), Some("fix the  bug"));
        assert!(cli.sandbox);
    }

    #[test]
    fn a_positional_prompt_conflicts_with_p() {
        let err = Cli::try_parse_from(["minima", "-p", "one", "two"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
        // --json alone needs -p; with words it must not be dropped silently.
        let err = Cli::try_parse_from(["minima", "--json"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        let err = Cli::try_parse_from(["minima", "--json", "two"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    /// ACP owns stdin and stdout, so no mode that reads or prints there can share the run.
    #[test]
    fn acp_refuses_the_other_modes() {
        assert!(Cli::try_parse_from(["minima", "--acp"]).unwrap().acp);
        for other in [
            &["-p", "x"][..],
            &["words"],
            &["-p", "x", "--json"],
            &["--resume", "1-2"],
            &["--continue"],
            &["-r", "--budget-tokens", "9"],
        ] {
            let args = ["minima", "--acp"].iter().chain(other);
            let err = Cli::try_parse_from(args).unwrap_err();
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{other:?}"
            );
        }
    }

    #[test]
    fn root_defaults_to_and_resolves_the_current_directory() {
        let root = cli(None, None, false).resolve_root().unwrap();
        assert_eq!(root, std::env::current_dir().unwrap());
    }

    /// Without the sandbox nothing bounds `bash`, so the flag would grant nothing. Refused rather
    /// than ignored: the user named a path and would otherwise believe it was granted.
    #[test]
    fn writable_without_the_sandbox_is_refused() {
        let mut c = cli(Some("m"), Some(64), false);
        c.writable = vec![std::env::temp_dir()];
        let err = c.bounds(std::env::temp_dir()).expect_err("should refuse");
        assert!(err.to_string().contains("--sandbox"), "{err}");
    }

    #[test]
    fn a_writable_path_that_does_not_resolve_is_refused() {
        let mut c = cli(Some("m"), Some(64), false);
        c.sandbox = true;
        c.writable = vec![PathBuf::from("/no/such/directory")];
        let err = format!("{:#}", c.bounds(std::env::temp_dir()).expect_err("refuses"));
        assert!(err.contains("--writable"), "{err}");
    }

    /// Seatbelt matches the resolved path, and on macOS `$TMPDIR` reaches it through
    /// `/var` -> `/private/var`, so resolution happens once here rather than per command.
    #[test]
    fn writable_paths_are_resolved_once() {
        let mut c = cli(Some("m"), Some(64), false);
        c.sandbox = true;
        c.writable = vec![std::env::temp_dir()];
        let bounds = c.bounds(std::env::temp_dir()).expect("resolves");
        assert_eq!(
            bounds.writable,
            vec![std::fs::canonicalize(std::env::temp_dir()).unwrap()]
        );
    }

    #[test]
    fn restrict_to_owner_strips_group_and_other() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("minima-perm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("history.txt");
        std::fs::write(&file, b"secret prompt").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

        restrict_to_owner(&file).unwrap();
        restrict_to_owner(&dir).unwrap();

        let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(&dir), 0o700);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn an_unknown_provider_lists_the_known_ones() {
        let mut c = cli(Some("m"), Some(64), false);
        c.provider = Some("notaprovider".into());
        let err = c.resolve().await.expect_err("should refuse").to_string();
        assert!(err.contains("notaprovider"), "{err}");
        assert!(err.contains("openrouter"), "{err}");
    }

    /// Refused before autoselect runs, so no environment can route the key to another vendor.
    #[tokio::test]
    async fn a_key_without_a_named_provider_is_refused() {
        let mut c = cli(Some("m"), Some(64), false);
        c.provider = None;
        let err = c.resolve().await.expect_err("should refuse").to_string();
        assert!(err.contains("--provider"), "{err}");
    }

    #[tokio::test]
    async fn the_provider_fixes_the_dialect_and_base_url() {
        let mut c = cli(Some("claude-haiku-4-5"), Some(64), false);
        c.provider = Some("anthropic".into());
        let resolved = c.resolve().await.expect("resolves");
        assert_eq!(resolved.dialect, Dialect::Messages);
        assert_eq!(resolved.base_url, "https://api.anthropic.com/v1");
    }

    /// Each library error that a flag fixes names that flag.
    #[test]
    fn resolve_errors_name_the_flag_that_fixes_them() {
        let cases = [
            (ResolveError::KeyWithoutProvider, "--provider"),
            (ResolveError::NoProvider, "--provider"),
            (
                ResolveError::NoKey {
                    provider: "openai",
                    var: "OPENAI_API_KEY",
                },
                "--api-key",
            ),
            (
                ResolveError::NoModel {
                    provider: "openai",
                    cached: 0,
                },
                "--model",
            ),
        ];
        for (e, flag) in cases {
            let text = flagged(e).to_string();
            assert!(text.contains(flag), "{text}");
        }
    }

    /// The escape hatch moves the endpoint without changing the shape spoken to it.
    #[tokio::test]
    async fn base_url_overrides_the_endpoint_but_not_the_dialect() {
        let mut c = cli(Some("m"), Some(64), false);
        c.provider = Some("anthropic".into());
        c.base_url = Some("http://127.0.0.1:9999/v1/".into());
        let resolved = c.resolve().await.expect("resolves");
        assert_eq!(resolved.base_url, "http://127.0.0.1:9999/v1");
        assert_eq!(resolved.dialect, Dialect::Messages);
    }
}
