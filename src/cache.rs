//! The model list, cached on disk, one file per endpoint.
//!
//! Session resume would reuse this file's atomic-write and schema-version handling.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::provider::Dialect;
use crate::provider::http::{authorize, client};

const TTL_SECS: u64 = 24 * 60 * 60;
/// How long a failed fetch holds off the next. A host that drops packets costs the connect
/// timeout on each attempt, and every start would pay it.
const RETRY_SECS: u64 = 60 * 60;
/// Bounds the cursor walk, so a gateway that always answers `has_more` cannot loop it forever.
const MAX_PAGES: usize = 20;
/// 2 added `pricing`, 3 `max_output`, 4 `adaptive_thinking`; an older file would hide the field
/// for a day.
const SCHEMA: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    /// Present only when the endpoint reports it; OpenAI's /models does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
    /// Anthropic's name for the context window. Folded into `context_length` on fetch.
    #[serde(default, skip_serializing)]
    max_input_tokens: Option<u32>,
    /// The model's output ceiling. Anthropic reports it as `max_tokens`, OpenRouter under
    /// `top_provider`; both are folded in on fetch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output: Option<u32>,
    #[serde(default, skip_serializing)]
    max_tokens: Option<u32>,
    #[serde(default, skip_serializing)]
    top_provider: Option<TopProvider>,
    /// Anthropic's model accepts `thinking: {type: "adaptive"}`. From `capabilities` on fetch.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub adaptive_thinking: bool,
    #[serde(default, skip_serializing)]
    capabilities: Option<serde_json::Value>,
    /// OpenRouter's per-token prices, kept as served and parsed by `price` on use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing: Option<serde_json::Value>,
}

impl Entry {
    /// Each endpoint's spelling of a limit, moved to the one field minima reads.
    fn normalise(&mut self) {
        self.context_length = self.context_length.or(self.max_input_tokens.take());
        let listed = self
            .top_provider
            .take()
            .and_then(|t| t.max_completion_tokens);
        self.max_output = self.max_output.or(self.max_tokens.take()).or(listed);
        if let Some(caps) = self.capabilities.take() {
            self.adaptive_thinking |= caps["thinking"]["types"]["adaptive"]["supported"] == true;
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TopProvider {
    max_completion_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
    schema: u32,
    fetched_at: u64,
    /// The last failed fetch, or 0. Keeps the entries of the last good one.
    #[serde(default)]
    failed_at: u64,
    endpoint: String,
    entries: Vec<Entry>,
}

impl Models {
    pub fn load(base_url: &str) -> Self {
        let empty = Self {
            schema: SCHEMA,
            fetched_at: 0,
            failed_at: 0,
            endpoint: base_url.to_string(),
            entries: Vec::new(),
        };
        let Some(path) = Self::path(base_url) else {
            return empty;
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return empty;
        };
        match serde_json::from_str::<Self>(&raw) {
            Ok(c) if c.schema == SCHEMA && c.endpoint == base_url => c,
            _ => empty,
        }
    }

    pub fn is_stale(&self) -> bool {
        self.entries.is_empty() || now().saturating_sub(self.fetched_at) > TTL_SECS
    }

    /// False within `RETRY_SECS` of a failed fetch.
    pub fn retry_due(&self) -> bool {
        now().saturating_sub(self.failed_at) > RETRY_SECS
    }

    /// Records a failed fetch, so the next start does not wait on the same host again.
    pub fn record_failure(&mut self) {
        self.failed_at = now();
        self.store();
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Only unambiguous when the endpoint serves one model, which is the llama.cpp and Ollama
    /// single-model case. Otherwise the user names it.
    pub fn default_model(&self) -> Option<String> {
        match self.entries.as_slice() {
            [only] => Some(only.id.clone()),
            _ => None,
        }
    }

    pub fn context_for(&self, model: &str) -> Option<u32> {
        self.entries.iter().find(|e| e.id == model)?.context_length
    }

    pub fn find(&self, model: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == model)
    }

    pub async fn refresh(&mut self, base_url: &str, api_key: &str, dialect: Dialect) -> Result<()> {
        #[derive(Deserialize)]
        struct Page {
            data: Vec<Entry>,
            /// Only Anthropic pages the list; OpenAI-shaped endpoints return it whole.
            #[serde(default)]
            has_more: bool,
            last_id: Option<String>,
        }

        let client = client()?;
        let base = format!("{}/models", base_url.trim_end_matches('/'));
        let mut entries = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut url = reqwest::Url::parse(&base).with_context(|| format!("parsing {base}"))?;
            if dialect == Dialect::Messages {
                // Anthropic's page size defaults to 20; 1000 is its maximum.
                url.query_pairs_mut().append_pair("limit", "1000");
            }
            if let Some(id) = &after {
                url.query_pairs_mut().append_pair("after_id", id);
            }
            let page: Page = authorize(client.get(url.clone()), dialect, api_key)
                .send()
                .await
                .with_context(|| format!("GET {url}"))?
                .error_for_status()?
                .json()
                .await
                .context("parsing the model list")?;

            entries.extend(page.data);
            match page.last_id {
                Some(id) if page.has_more && after.as_ref() != Some(&id) => after = Some(id),
                _ => break,
            }
        }

        entries.iter_mut().for_each(Entry::normalise);
        self.entries = entries;
        self.fetched_at = now();
        self.failed_at = 0;
        self.endpoint = base_url.to_string();
        self.store();
        Ok(())
    }

    /// Best effort. A cache that cannot be written is not a reason to fail the run.
    fn store(&self) {
        let Some(path) = Self::path(&self.endpoint) else {
            return;
        };
        let Some(parent) = path.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(body) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let _ = crate::config::restrict_to_owner(parent);
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, body).is_ok() {
            // Tightened before the rename, not after: restricting the destination would leave a
            // window where the file is readable at the default umask.
            let _ = crate::config::restrict_to_owner(&tmp);
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    /// One cache file per endpoint, named by a hash so the path stays filesystem-safe.
    fn path(base_url: &str) -> Option<PathBuf> {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in base_url.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
        Some(crate::config::config_dir()?.join(format!("models-{h:016x}.json")))
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(json: &str) -> Entry {
        let mut entry: Entry = serde_json::from_str(json).unwrap();
        entry.normalise();
        entry
    }

    #[test]
    fn each_endpoints_limits_land_in_one_field() {
        let anthropic = entry(r#"{"id": "c", "max_input_tokens": 200000, "max_tokens": 64000}"#);
        assert_eq!(
            (anthropic.context_length, anthropic.max_output),
            (Some(200_000), Some(64_000))
        );

        let openrouter = entry(
            r#"{"id": "o", "context_length": 400000,
                "top_provider": {"max_completion_tokens": 128000}}"#,
        );
        assert_eq!(
            (openrouter.context_length, openrouter.max_output),
            (Some(400_000), Some(128_000))
        );

        let thinking = entry(
            r#"{"id": "c", "capabilities": {"thinking": {"types": {"adaptive": {"supported": true}}}}}"#,
        );
        assert!(thinking.adaptive_thinking);
        let stored = serde_json::to_string(&thinking).unwrap();
        assert!(entry(&stored).adaptive_thinking, "{stored}");

        let openai = entry(r#"{"id": "g"}"#);
        assert_eq!((openai.context_length, openai.max_output), (None, None));
    }

    /// The cache file keeps only the folded fields, so a reload does not depend on the fold.
    #[test]
    fn a_stored_entry_round_trips_with_its_limits() {
        let stored = serde_json::to_string(&entry(r#"{"id": "c", "max_tokens": 64000}"#)).unwrap();
        assert_eq!(entry(&stored).max_output, Some(64_000));
    }

    #[test]
    fn a_failed_fetch_holds_off_the_next_one() {
        let mut list = Models {
            schema: SCHEMA,
            fetched_at: 0,
            failed_at: 0,
            endpoint: "http://x".into(),
            entries: Vec::new(),
        };
        assert!(list.is_stale() && list.retry_due());
        list.failed_at = now();
        assert!(list.is_stale() && !list.retry_due());
        list.failed_at = now() - RETRY_SECS - 1;
        assert!(list.retry_due());
    }
}
