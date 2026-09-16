//! Flags, then environment, then the provider registry, then the model cache. No config format.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Parser;

use crate::provider::{Dialect, registry};

/// Tool results are truncated to this many bytes before they enter the context.
pub const TOOL_OUTPUT_CAP: usize = 32 * 1024;

/// Tokens of headroom below the model's context window before the turn is refused.
pub const CONTEXT_MARGIN: u32 = 2048;

#[derive(Parser, Debug, Clone)]
#[command(name = "minima", version, about)]
pub struct Cli {
    /// Headless: answer this prompt, print the result, exit.
    #[arg(short = 'p', long, value_name = "TEXT")]
    pub prompt: Option<String>,

    /// With -p: print one JSON record per line on stdout, ending in a `result` record.
    #[arg(long, requires = "prompt")]
    pub json: bool,

    /// Which provider to talk to: fixes the endpoint, the wire format and the key variable.
    /// Left out, minima takes the first provider whose key variable is set.
    #[arg(long, env = "MINIMA_PROVIDER", value_name = "ID")]
    pub provider: Option<String>,

    /// Left out, minima reuses the model last used with this provider.
    #[arg(long, env = "MINIMA_MODEL", value_name = "ID")]
    pub model: Option<String>,

    /// Override the provider's endpoint, for a local server or a gateway. Never changes the wire
    /// format: a different shape is a different provider, not a different address.
    #[arg(long, env = "MINIMA_BASE_URL", value_name = "URL")]
    pub base_url: Option<String>,

    /// Overrides the provider's key variable. Requires --provider.
    #[arg(
        long,
        env = "MINIMA_API_KEY",
        hide_env_values = true,
        value_name = "KEY"
    )]
    pub api_key: Option<String>,

    /// Print without colour. Colour is off anyway when stdout is not a terminal, or when
    /// NO_COLOR is set.
    #[arg(long)]
    pub no_color: bool,

    /// Context window in tokens. Falls back to the cached value for the model.
    #[arg(long, env = "MINIMA_CONTEXT", value_name = "N")]
    pub context: Option<u32>,

    /// Replay a scripted JSON stream instead of calling the network.
    #[arg(long, value_name = "PATH")]
    pub mock: Option<PathBuf>,

    /// Refuse to keep going after this many provider round-trips in one user turn.
    #[arg(long, default_value_t = 32, value_name = "N")]
    pub max_turns: u32,

    /// Re-fetch the model list even if the cache is fresh.
    #[arg(long)]
    pub refresh_models: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// The registry id that was named or autoselected. Carried so the caller can record the
    /// pairing; resolution itself stays free of disk side effects.
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub dialect: Dialect,
    pub context: u32,
    pub max_turns: u32,
}

impl Config {
    #[cfg(test)]
    pub fn for_test(model: &str) -> Self {
        Self {
            provider: "openrouter".into(),
            base_url: "http://x/v1".into(),
            api_key: "k".into(),
            model: model.into(),
            dialect: Dialect::Chat,
            context: 128_000,
            max_turns: 8,
        }
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
                context: self.context.unwrap_or(128_000),
                max_turns: self.max_turns,
            });
        }

        // A key does not say which vendor issued it, so autoselect could send it to another one.
        if self.api_key.is_some() && self.provider.is_none() {
            bail!("--api-key needs --provider: a key does not say which provider it belongs to");
        }

        let entry = match &self.provider {
            Some(named) => match registry::find(named) {
                Some(entry) => entry,
                None => bail!(
                    "unknown provider {named:?}; known: {}",
                    registry::ids().join(", ")
                ),
            },
            // No provider named: take the first whose key variable is set. Local servers carry
            // no key and so are never autoselected.
            None => match registry::autoselect(|var| std::env::var(var).ok()) {
                Some(entry) => entry,
                None => bail!(
                    "no provider key found; set one of {}, or pass --provider",
                    registry::key_vars()
                        .iter()
                        .map(|v| format!("${v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
        };

        let base_url = self
            .base_url
            .clone()
            .unwrap_or_else(|| entry.base_url.to_string())
            .trim_end_matches('/')
            .to_string();

        // A local server wants no credential, so an absent key is only an error when the entry
        // names a variable for one.
        let api_key = match self.api_key.clone() {
            Some(key) => key,
            None => match entry.key_env {
                Some(var) => match std::env::var(var) {
                    Ok(key) if !key.is_empty() => key,
                    _ => bail!("no key for {}: set ${var} or pass --api-key", entry.id),
                },
                None => String::new(),
            },
        };

        let mut cache = crate::cache::Models::load(&base_url);
        if self.wants_model_list(cache.is_stale())
            && let Err(e) = cache.refresh(&base_url, &api_key, entry.dialect).await
        {
            // Plenty of endpoints do not serve /models. That must not stop minima when the user
            // already named the model.
            if self.model.is_none() || self.refresh_models {
                return Err(e);
            }
            tracing::warn!("model list unavailable, continuing: {e:#}");
        }

        // Explicit beats remembered beats a single-model endpoint. The pairing is written by
        // the caller, not here: resolving config should not touch the disk.
        let model = match self
            .model
            .clone()
            .or_else(|| crate::state::State::load().last_model(entry.id))
            .or_else(|| cache.default_model())
        {
            Some(m) => m,
            None => bail!(
                "no model for {}: pass --model or set MINIMA_MODEL ({} cached)",
                entry.id,
                cache.count()
            ),
        };

        Ok(Config {
            context: self
                .context
                .or_else(|| cache.context_for(&model))
                .unwrap_or(128_000),
            provider: entry.id.to_string(),
            base_url,
            api_key,
            model,
            dialect: entry.dialect,
            max_turns: self.max_turns,
        })
    }

    /// A round-trip before the first prompt is only justified by a field the flags left unset.
    fn wants_model_list(&self, cache_stale: bool) -> bool {
        self.refresh_models || ((self.model.is_none() || self.context.is_none()) && cache_stale)
    }
}

/// Prompt history. Whatever the user typed goes here verbatim, so it is owner-only.
pub fn history_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("history.txt"))
}

/// Tighten a path minima created to owner-only: 0700 for a directory, 0600 for a file. History and
/// the model cache both sit under the config directory, and session resume will land there too.
#[cfg(unix)]
pub fn restrict_to_owner(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mode = if metadata.is_dir() { 0o700 } else { 0o600 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
pub fn restrict_to_owner(_path: &std::path::Path) -> std::io::Result<()> {
    Ok(())
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
            prompt: None,
            json: false,
            provider: Some("openrouter".into()),
            model: model.map(String::from),
            base_url: None,
            api_key: Some("k".into()),
            no_color: false,
            context,
            mock: None,
            max_turns: 32,
            refresh_models: refresh,
        }
    }

    #[test]
    fn fully_specified_run_skips_the_model_list() {
        assert!(!cli(Some("m"), Some(64), false).wants_model_list(true));
    }

    #[test]
    fn a_missing_field_fetches_only_once_the_cache_is_stale() {
        assert!(cli(None, Some(64), false).wants_model_list(true));
        assert!(cli(Some("m"), None, false).wants_model_list(true));
        assert!(!cli(None, Some(64), false).wants_model_list(false));
    }

    #[test]
    fn an_explicit_refresh_ignores_both() {
        assert!(cli(Some("m"), Some(64), true).wants_model_list(false));
    }

    #[cfg(unix)]
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
