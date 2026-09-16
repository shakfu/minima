//! The model list, cached on disk, one file per endpoint.
//!
//! Session resume would reuse this file's atomic-write and schema-version handling.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const TTL_SECS: u64 = 24 * 60 * 60;
const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    /// Present only when the endpoint reports it; OpenAI's /models does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
    schema: u32,
    fetched_at: u64,
    endpoint: String,
    entries: Vec<Entry>,
}

impl Models {
    pub fn load(base_url: &str) -> Self {
        let empty = Self {
            schema: SCHEMA,
            fetched_at: 0,
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

    pub async fn refresh(&mut self, base_url: &str, api_key: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct List {
            data: Vec<Entry>,
        }

        let url = format!("{}/models", base_url.trim_end_matches('/'));
        let list: List = crate::provider::http::client()?
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?
            .error_for_status()?
            .json()
            .await
            .context("parsing the model list")?;

        self.entries = list.data;
        self.fetched_at = now();
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
