//! From what the caller knows -- a provider, a model, perhaps neither -- to a `Config`, through
//! the registry and the provider's model list. Where the values come from (flags, environment, a
//! settings file) is the caller's concern, and so is remembering the choice afterwards: resolving
//! writes nothing but the model list cache.

use std::path::PathBuf;

use crate::cache::Models;
use crate::config::Config;
use crate::provider::registry::{self, Entry};

/// The context window assumed when neither the caller nor a model list gives one.
pub const FALLBACK_CONTEXT: u32 = 128_000;

/// Provider round-trips allowed per user turn when the caller sets none.
pub const DEFAULT_MAX_TURNS: u32 = 100;
const _: () = assert!(DEFAULT_MAX_TURNS > 0);

#[derive(Debug, Clone)]
pub struct Request {
    /// A registry id. `None` autoselects; see `registry::autoselect`.
    pub provider: Option<String>,
    pub model: Option<String>,
    /// Moves the endpoint, never the dialect.
    pub base_url: Option<String>,
    /// Takes precedence over the key the registry entry names. Needs a named provider. A blank
    /// key counts as none.
    pub api_key: Option<String>,
    pub context: Option<u32>,
    pub max_turns: u32,
    /// Fetch the model list even when the cache is fresh.
    pub refresh_models: bool,
    /// Where model lists are cached; `None` caches nothing.
    pub cache_dir: Option<PathBuf>,
    /// The provider last used, tried first when none is named.
    pub last_provider: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// A key does not say which vendor issued it, so autoselect could send it to another one.
    #[error("an API key needs a named provider: a key does not say which provider it belongs to")]
    KeyWithoutProvider,
    #[error("unknown provider {0:?}; known: {known}", known = registry::ids().join(", "))]
    UnknownProvider(String),
    /// No provider named, and no registry entry's key is set.
    #[error("no provider key found; set one of {}", vars(registry::key_vars()))]
    NoProvider,
    #[error("no key for {provider}: set ${var}")]
    NoKey {
        provider: &'static str,
        var: &'static str,
    },
    /// No model named or remembered, and the endpoint does not serve exactly one.
    #[error("no model for {provider} ({cached} in its model list)")]
    NoModel {
        provider: &'static str,
        cached: usize,
    },
    #[error("fetching the model list")]
    ModelList(#[source] anyhow::Error),
}

fn vars(names: Vec<&str>) -> String {
    names
        .iter()
        .map(|v| format!("${v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Default for Request {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            base_url: None,
            api_key: None,
            context: None,
            max_turns: DEFAULT_MAX_TURNS,
            refresh_models: false,
            cache_dir: None,
            last_provider: None,
        }
    }
}

/// A key with surrounding whitespace removed; a blank one is none.
fn usable(key: &str) -> Option<String> {
    Some(key.trim()).filter(|k| !k.is_empty()).map(String::from)
}

impl Request {
    /// `key` reads a key variable, such as the environment does; `last_model` gives the model
    /// last used with a provider.
    pub async fn resolve(
        &self,
        key: impl Fn(&str) -> Option<String>,
        last_model: impl Fn(&str) -> Option<String>,
    ) -> Result<Config, ResolveError> {
        let explicit = self.api_key.as_deref().and_then(usable);
        if explicit.is_some() && self.provider.is_none() {
            return Err(ResolveError::KeyWithoutProvider);
        }
        let entry: &Entry = match &self.provider {
            Some(named) => {
                registry::find(named).ok_or_else(|| ResolveError::UnknownProvider(named.clone()))?
            }
            // Local servers carry no key and so are never autoselected.
            None => registry::autoselect(self.last_provider.as_deref(), &key)
                .ok_or(ResolveError::NoProvider)?,
        };

        let base_url = self
            .base_url
            .as_deref()
            .unwrap_or(entry.base_url)
            .trim_end_matches('/')
            .to_string();

        // A local server wants no credential, so an absent key is only an error when the entry
        // names a variable for one.
        let api_key = match (explicit, entry.key_env) {
            (Some(key), _) => key,
            (None, Some(var)) => {
                key(var)
                    .as_deref()
                    .and_then(usable)
                    .ok_or(ResolveError::NoKey {
                        provider: entry.id,
                        var,
                    })?
            }
            (None, None) => String::new(),
        };

        let mut cache = Models::load(self.cache_dir.as_deref(), &base_url);
        if self.wants_model_list(cache.is_stale())
            && let Err(e) = cache.refresh(&base_url, &api_key, entry.dialect).await
        {
            // Plenty of endpoints do not serve /models. That must not stop a run whose model
            // is already named.
            if self.model.is_none() || self.refresh_models {
                return Err(ResolveError::ModelList(e));
            }
            tracing::warn!("model list unavailable, continuing: {e:#}");
        }

        // Named beats remembered beats a single-model endpoint.
        let model = self
            .model
            .clone()
            .or_else(|| last_model(entry.id))
            .or_else(|| cache.default_model())
            .ok_or(ResolveError::NoModel {
                provider: entry.id,
                cached: cache.count(),
            })?;

        let listed = cache.find(&model);
        let context = self.context.or_else(|| cache.context_for(&model));
        Ok(Config {
            context: context.unwrap_or(FALLBACK_CONTEXT),
            context_guessed: context.is_none(),
            max_output: listed.and_then(|e| e.max_output),
            adaptive_thinking: listed.is_some_and(|e| e.adaptive_thinking),
            provider: entry.id.to_string(),
            base_url,
            api_key,
            model,
            dialect: entry.dialect,
            max_turns: self.max_turns,
            pricing: None,
            rates: None,
            cap_output: false,
            cache_dir: self.cache_dir.clone(),
        })
    }

    /// A round-trip before the first prompt is only justified by a field the request left unset.
    pub fn wants_model_list(&self, cache_stale: bool) -> bool {
        self.refresh_models || ((self.model.is_none() || self.context.is_none()) && cache_stale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Dialect;

    fn request(model: Option<&str>, context: Option<u32>, refresh: bool) -> Request {
        Request {
            provider: Some("openrouter".into()),
            model: model.map(String::from),
            api_key: Some("k".into()),
            context,
            refresh_models: refresh,
            ..Request::default()
        }
    }

    fn none(_: &str) -> Option<String> {
        None
    }

    async fn resolve(r: &Request) -> Result<Config, ResolveError> {
        r.resolve(none, none).await
    }

    #[test]
    fn fully_specified_run_skips_the_model_list() {
        assert!(!request(Some("m"), Some(64), false).wants_model_list(true));
    }

    #[test]
    fn a_missing_field_fetches_only_once_the_cache_is_stale() {
        assert!(request(None, Some(64), false).wants_model_list(true));
        assert!(request(Some("m"), None, false).wants_model_list(true));
        assert!(!request(None, Some(64), false).wants_model_list(false));
    }

    #[test]
    fn an_explicit_refresh_ignores_both() {
        assert!(request(Some("m"), Some(64), true).wants_model_list(false));
    }

    #[tokio::test]
    async fn an_unknown_provider_lists_the_known_ones() {
        let mut r = request(Some("m"), Some(64), false);
        r.provider = Some("notaprovider".into());
        let err = resolve(&r).await.expect_err("should refuse").to_string();
        assert!(err.contains("notaprovider"), "{err}");
        assert!(err.contains("openrouter"), "{err}");
    }

    /// Refused before autoselect runs, so no environment can route the key to another vendor.
    #[tokio::test]
    async fn a_key_without_a_named_provider_is_refused() {
        let mut r = request(Some("m"), Some(64), false);
        r.provider = None;
        let err = resolve(&r).await.expect_err("should refuse");
        assert!(matches!(err, ResolveError::KeyWithoutProvider), "{err}");
    }

    #[tokio::test]
    async fn the_provider_fixes_the_dialect_and_base_url() {
        let mut r = request(Some("claude-haiku-4-5"), Some(64), false);
        r.provider = Some("anthropic".into());
        let resolved = resolve(&r).await.expect("resolves");
        assert_eq!(resolved.dialect, Dialect::Messages);
        assert_eq!(resolved.base_url, "https://api.anthropic.com/v1");
    }

    /// The escape hatch moves the endpoint without changing the shape spoken to it.
    #[tokio::test]
    async fn base_url_overrides_the_endpoint_but_not_the_dialect() {
        let mut r = request(Some("m"), Some(64), false);
        r.provider = Some("anthropic".into());
        r.base_url = Some("http://127.0.0.1:9999/v1/".into());
        let resolved = resolve(&r).await.expect("resolves");
        assert_eq!(resolved.base_url, "http://127.0.0.1:9999/v1");
        assert_eq!(resolved.dialect, Dialect::Messages);
    }

    /// The caller's key source, not the process environment, supplies the key; an empty one is
    /// no key.
    #[tokio::test]
    async fn the_key_comes_from_the_callers_source() {
        let mut r = request(Some("m"), Some(64), false);
        r.api_key = None;
        let found = r
            .resolve(
                |var| (var == "OPENROUTER_API_KEY").then(|| "sk".into()),
                none,
            )
            .await
            .expect("resolves");
        assert_eq!(found.api_key, "sk");

        let err = r
            .resolve(|_| Some(" ".into()), none)
            .await
            .expect_err("blank");
        assert!(
            matches!(
                err,
                ResolveError::NoKey {
                    var: "OPENROUTER_API_KEY",
                    ..
                }
            ),
            "{err}"
        );
    }

    /// A default request can run turns.
    #[test]
    fn the_default_allows_turns() {
        assert_eq!(Request::default().max_turns, DEFAULT_MAX_TURNS);
    }

    /// Keys from either source are trimmed, and a blank explicit key falls back to the source.
    #[tokio::test]
    async fn keys_are_trimmed_and_a_blank_one_is_none() {
        let mut r = request(Some("m"), Some(64), false);
        r.api_key = Some(" sk \n".into());
        assert_eq!(resolve(&r).await.unwrap().api_key, "sk");

        r.api_key = None;
        let env = r
            .resolve(|_| Some("sk\n".into()), none)
            .await
            .expect("resolves");
        assert_eq!(env.api_key, "sk");

        r.api_key = Some("  ".into());
        let err = resolve(&r).await.expect_err("blank");
        assert!(matches!(err, ResolveError::NoKey { .. }), "{err}");

        // A blank key is no key, so it does not need a named provider.
        r.provider = None;
        let err = resolve(&r).await.expect_err("no provider");
        assert!(matches!(err, ResolveError::NoProvider), "{err}");
    }

    /// The fetch failure stays reachable as the error's source.
    #[test]
    fn a_model_list_failure_keeps_its_cause() {
        use std::error::Error;
        let err = ResolveError::ModelList(anyhow::anyhow!("401"));
        assert_eq!(err.source().unwrap().to_string(), "401");
        let shown = format!("{:#}", anyhow::Error::from(err));
        assert_eq!(shown, "fetching the model list: 401");
    }

    /// A blank key is no key, so autoselect finds no provider.
    #[tokio::test]
    async fn a_blank_environment_key_is_an_error() {
        let mut r = request(Some("m"), Some(64), false);
        r.provider = None;
        r.api_key = None;
        let blank = |var: &str| (var == "OPENROUTER_API_KEY").then(|| " \n".to_string());
        let err = r.resolve(blank, none).await.expect_err("blank key");
        assert!(matches!(err, ResolveError::NoProvider), "{err}");
    }

    /// Autoselect tries the last provider first, and a local server needs no key.
    #[tokio::test]
    async fn autoselect_prefers_the_last_provider_and_local_servers_need_no_key() {
        let mut r = request(Some("m"), Some(64), false);
        r.provider = None;
        r.api_key = None;
        r.last_provider = Some("anthropic".into());
        let any = |_: &str| Some("sk".to_string());
        assert_eq!(r.resolve(any, none).await.unwrap().provider, "anthropic");

        let err = r.resolve(none, none).await.expect_err("no key set");
        assert!(matches!(err, ResolveError::NoProvider), "{err}");

        r.provider = Some("llamacpp".into());
        let local = r.resolve(none, none).await.expect("no key needed");
        assert_eq!(local.api_key, "");
    }
}
