//! A resolved run configuration; `resolve` builds one.

use std::path::PathBuf;

use crate::provider::Dialect;

/// Tool results are truncated to this many bytes before they enter the context.
pub const TOOL_OUTPUT_CAP: usize = 32 * 1024;

/// Tokens of headroom below the model's context window before the turn is refused.
pub const CONTEXT_MARGIN: u32 = 2048;

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
    /// True when no flag or model list gave the window, so `context` is the 128k fallback.
    pub context_guessed: bool,
    /// The model's output ceiling, where a model list reports it.
    pub max_output: Option<u32>,
    /// The model list says the model takes adaptive thinking; see `anthropic::build_body`.
    pub adaptive_thinking: bool,
    pub max_turns: u32,
    /// Set only when the provider reports no cost; see `price::lookup`.
    pub pricing: Option<crate::price::Pricing>,
    /// The listed rates, whether or not the provider reports cost: a budget projects each
    /// request's cost from them before sending it.
    pub rates: Option<crate::price::Pricing>,
    /// Send an output cap on every dialect, not only Messages: set under a budget, which grants
    /// each request its output.
    pub cap_output: bool,
    /// Where model lists are cached; `None` caches nothing. See `cache::Models`.
    pub cache_dir: Option<PathBuf>,
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
            context_guessed: false,
            max_output: None,
            adaptive_thinking: false,
            max_turns: 8,
            pricing: None,
            rates: None,
            cap_output: false,
            cache_dir: None,
        }
    }
}
