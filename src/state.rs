//! What minima remembers between runs: the model last used with each provider.
//!
//! Separate from `cache.rs` on purpose. The model list is a cache, disposable and keyed by
//! endpoint; this is a preference, meaningful and keyed by provider, and it survives a
//! `--base-url` override that would change the cache's filename.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const SCHEMA: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    schema: u32,
    /// provider id -> model id
    #[serde(default)]
    last_model: BTreeMap<String, String>,
}

impl State {
    pub fn load() -> Self {
        Self::path().map_or_else(Self::default, |path| Self::load_from(&path))
    }

    fn load_from(path: &std::path::Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&raw) {
            Ok(state) if state.schema == SCHEMA => state,
            // A file from another schema is discarded rather than migrated; the cost of losing
            // it is one `--model`.
            _ => Self::default(),
        }
    }

    pub fn last_model(&self, provider: &str) -> Option<String> {
        self.last_model.get(provider).cloned()
    }

    /// Records the pairing and writes it, unless it is already what is on disk.
    pub fn remember(&mut self, provider: &str, model: &str) {
        if self.last_model.get(provider).is_some_and(|m| m == model) {
            return;
        }
        self.last_model
            .insert(provider.to_string(), model.to_string());
        self.schema = SCHEMA;
        self.store();
    }

    /// Best effort. State that cannot be written is not a reason to fail the run.
    fn store(&self) {
        let Some(path) = Self::path() else { return };
        let Some(parent) = path.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let _ = crate::config::restrict_to_owner(parent);
        let Ok(body) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, body).is_ok() {
            // Tightened before the rename, so the file is never briefly world-readable.
            let _ = crate::config::restrict_to_owner(&tmp);
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    fn path() -> Option<std::path::PathBuf> {
        crate::config::config_dir().map(|d| d.join("state.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_per_provider_not_globally() {
        let mut state = State::default();
        state
            .last_model
            .insert("anthropic".into(), "claude-haiku-4-5".into());
        state
            .last_model
            .insert("openai".into(), "gpt-5-nano".into());

        assert_eq!(
            state.last_model("anthropic").as_deref(),
            Some("claude-haiku-4-5")
        );
        assert_eq!(state.last_model("openai").as_deref(), Some("gpt-5-nano"));
        assert_eq!(state.last_model("openrouter"), None);
    }

    #[test]
    fn a_file_from_another_schema_is_discarded_not_misread() {
        let dir = crate::tools::Scratch::new("state-schema");
        let path = dir.file("state.json");

        std::fs::write(&path, r#"{"schema":1,"last_model":{"openai":"gpt-5"}}"#).unwrap();
        let current = State::load_from(path.as_ref());
        assert_eq!(current.last_model("openai").as_deref(), Some("gpt-5"));

        std::fs::write(&path, r#"{"schema":99,"last_model":{"openai":"gpt-4"}}"#).unwrap();
        assert_eq!(State::load_from(path.as_ref()).last_model("openai"), None);
    }
}
