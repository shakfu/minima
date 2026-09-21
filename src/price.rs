//! Cost estimates for providers that report none, from OpenRouter's public price list.
//!
//! OpenAI and Anthropic return token counts but no cost. OpenRouter lists their models at the
//! vendors' rates, and its `/models` needs no key. A table kept in minima would go stale.

use serde_json::Value;

use crate::cache::Models;
use crate::config::Config;
use crate::provider::{Dialect, Usage, registry};

/// USD per token.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rates {
    prompt: f64,
    completion: f64,
    cache_read: f64,
    cache_write: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pricing {
    base: Rates,
    /// `(min_prompt_tokens, rates)`. Some models charge more for a long prompt.
    tiers: Vec<(u64, Rates)>,
}

impl Pricing {
    /// OpenRouter's `pricing` object, whose figures are decimal strings. None for a model priced
    /// per route (`-1`), which has no fixed rate to estimate from.
    pub fn from_openrouter(pricing: &Value) -> Option<Self> {
        let base = rates(pricing, None)?;
        let tiers = pricing["overrides"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| Some((t["min_prompt_tokens"].as_u64()?, rates(t, Some(base))?)))
            .collect();
        Some(Self { base, tiers })
    }

    pub fn cost(&self, usage: &Usage) -> f64 {
        let prompt = u64::from(usage.prompt_tokens);
        let r = self
            .tiers
            .iter()
            .filter(|(min, _)| prompt >= *min)
            .max_by_key(|(min, _)| *min)
            .map_or(self.base, |(_, r)| *r);
        let fresh = usage
            .prompt_tokens
            .saturating_sub(usage.cache_read + usage.cache_write);
        f64::from(fresh) * r.prompt
            + f64::from(usage.cache_read) * r.cache_read
            + f64::from(usage.cache_write) * r.cache_write
            + f64::from(usage.completion_tokens) * r.completion
    }
}

/// A tier lists only what it changes; the rest comes from `fallback`. Cache rates missing from
/// both default to the prompt rate.
fn rates(v: &Value, fallback: Option<Rates>) -> Option<Rates> {
    let get = |key: &str| -> Option<f64> {
        let rate = v[key].as_str()?.parse::<f64>().ok()?;
        (rate >= 0.0).then_some(rate)
    };
    let prompt = get("prompt").or(fallback.map(|f| f.prompt))?;
    let completion = get("completion").or(fallback.map(|f| f.completion))?;
    Some(Rates {
        prompt,
        completion,
        cache_read: get("input_cache_read")
            .or(fallback.map(|f| f.cache_read))
            .unwrap_or(prompt),
        cache_write: get("input_cache_write")
            .or(fallback.map(|f| f.cache_write))
            .unwrap_or(prompt),
    })
}

/// Prices for the configured model, when its provider reports no cost of its own. A failed fetch
/// loses only the estimate, so it never stops the run.
pub async fn estimate(config: &Config, refresh: bool) -> Option<Pricing> {
    // A gateway or proxy need not bill at the vendor's rates.
    let entry = registry::find(&config.provider)?;
    if config.base_url != entry.base_url {
        return None;
    }
    let ids = openrouter_ids(&config.provider, &config.model);
    if ids.is_empty() {
        return None;
    }
    let base = registry::find("openrouter")?.base_url;
    let mut list = Models::load(base);
    if (refresh || list.is_stale())
        && let Err(e) = list.refresh(base, "", Dialect::Chat).await
    {
        tracing::warn!("price list unavailable: {e:#}");
    }
    ids.iter().find_map(|id| list.pricing_for(id))
}

/// OpenRouter's candidate ids for a model: as given, then without a date suffix. Anthropic writes
/// `claude-haiku-4-5-20251001` where OpenRouter writes `claude-haiku-4.5`.
fn openrouter_ids(provider: &str, model: &str) -> Vec<String> {
    if !matches!(provider, "openai" | "anthropic") {
        return Vec::new();
    }
    let undated = strip_date(model);
    let mut ids = vec![model.to_string(), undated.to_string()];
    if provider == "anthropic" {
        let parts: Vec<&str> = undated.rsplitn(3, '-').collect();
        if let [minor, major, stem] = parts[..]
            && [minor, major].iter().all(|p| is_version(p))
        {
            ids.push(format!("{stem}-{major}.{minor}"));
        }
    }
    ids.dedup();
    ids.into_iter()
        .map(|id| format!("{provider}/{id}"))
        .collect()
}

fn is_version(part: &str) -> bool {
    (1..=2).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit())
}

/// Drops `-20251001` or `-2025-08-07`.
fn strip_date(model: &str) -> &str {
    let is_date = |tail: &[u8]| match tail.len() {
        8 => tail.iter().all(u8::is_ascii_digit),
        10 => tail.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        }),
        _ => false,
    };
    let bytes = model.as_bytes();
    for len in [8, 10] {
        // The byte at `cut` is an ASCII dash, so it is a char boundary.
        if let Some(cut) = bytes.len().checked_sub(len + 1)
            && bytes[cut] == b'-'
            && is_date(&bytes[cut + 1..])
        {
            return &model[..cut];
        }
    }
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn candidate_ids_cover_snapshots_and_anthropic_versions() {
        assert_eq!(
            openrouter_ids("anthropic", "claude-haiku-4-5-20251001"),
            [
                "anthropic/claude-haiku-4-5-20251001",
                "anthropic/claude-haiku-4-5",
                "anthropic/claude-haiku-4.5"
            ]
        );
        assert_eq!(
            openrouter_ids("anthropic", "claude-sonnet-5"),
            ["anthropic/claude-sonnet-5"]
        );
        assert_eq!(
            openrouter_ids("openai", "gpt-5-2025-08-07"),
            ["openai/gpt-5-2025-08-07", "openai/gpt-5"]
        );
        assert_eq!(
            openrouter_ids("openai", "gpt-5.6-luna"),
            ["openai/gpt-5.6-luna"]
        );
        // OpenRouter reports its own cost, and local servers charge nothing.
        assert!(openrouter_ids("openrouter", "openai/gpt-5").is_empty());
        assert!(openrouter_ids("ollama", "llama3").is_empty());
    }

    /// The shape OpenRouter lists `openai/gpt-5.6-luna` with.
    fn luna() -> Pricing {
        Pricing::from_openrouter(&json!({
            "prompt": "0.0000002", "completion": "0.0000012",
            "input_cache_read": "0.00000002", "input_cache_write": "0.00000025",
            "overrides": [{"min_prompt_tokens": 272000, "prompt": "0.0000004",
                           "completion": "0.0000018", "input_cache_read": "0.00000004"}]
        }))
        .expect("parses")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn cached_tokens_are_billed_at_the_cache_rate() {
        let usage = Usage {
            cache_read: 30_000,
            ..Usage::from_parts(40_000, 1_000)
        };
        // 10k fresh at 0.2/M, 30k cached at 0.02/M, 1k out at 1.2/M.
        assert!(close(luna().cost(&usage), 0.002 + 0.0006 + 0.0012));
    }

    #[test]
    fn a_long_prompt_uses_its_tier_and_inherits_what_the_tier_omits() {
        let p = luna();
        let usage = Usage::from_parts(300_000, 0);
        assert!(close(p.cost(&usage), 300_000.0 * 0.0000004));
        assert!(close(p.tiers[0].1.cache_write, 0.00000025));
    }

    #[test]
    fn a_variable_price_is_no_price() {
        assert!(Pricing::from_openrouter(&json!({"prompt": "-1", "completion": "-1"})).is_none());
        assert!(Pricing::from_openrouter(&json!({})).is_none());
    }
}
