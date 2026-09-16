//! The shipped providers. Pure data: a base URL, a dialect, and where the key comes from.
//!
//! `ALL` is in autoselect order. With no `--provider`, minima takes the first entry whose key
//! variable is set, so the order of this table is a user-visible decision and is tested as one.
//!
//! `--base-url` overrides the URL for a local server or a gateway. It never overrides the
//! dialect, because a different wire shape is a different provider, not a different address.

use super::Dialect;

pub struct Entry {
    pub id: &'static str,
    pub base_url: &'static str,
    pub dialect: Dialect,
    /// None for a local server that wants no credential.
    pub key_env: Option<&'static str>,
}

pub const ALL: &[Entry] = &[
    Entry {
        id: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        dialect: Dialect::Chat,
        key_env: Some("OPENROUTER_API_KEY"),
    },
    Entry {
        id: "anthropic",
        base_url: "https://api.anthropic.com/v1",
        dialect: Dialect::Messages,
        key_env: Some("ANTHROPIC_API_KEY"),
    },
    Entry {
        id: "openai",
        base_url: "https://api.openai.com/v1",
        dialect: Dialect::Responses,
        key_env: Some("OPENAI_API_KEY"),
    },
    // Local servers need no key, so they would match every autoselect. Reachable only by
    // naming them.
    Entry {
        id: "ollama",
        base_url: "http://127.0.0.1:11434/v1",
        dialect: Dialect::Chat,
        key_env: None,
    },
    Entry {
        id: "llamacpp",
        base_url: "http://127.0.0.1:8080/v1",
        dialect: Dialect::Chat,
        key_env: None,
    },
];

/// The first entry whose key variable holds something. `lookup` is injected so the order can be
/// tested without mutating the process environment.
pub fn autoselect(lookup: impl Fn(&str) -> Option<String>) -> Option<&'static Entry> {
    ALL.iter().find(|entry| {
        entry
            .key_env
            .and_then(&lookup)
            .is_some_and(|key| !key.trim().is_empty())
    })
}

/// The variables autoselect consults, in order, for the message when none of them is set.
pub fn key_vars() -> Vec<&'static str> {
    ALL.iter().filter_map(|e| e.key_env).collect()
}

pub fn find(id: &str) -> Option<&'static Entry> {
    ALL.iter().find(|e| e.id == id)
}

pub fn ids() -> Vec<&'static str> {
    ALL.iter().map(|e| e.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_findable_and_unique() {
        let mut seen = Vec::new();
        for entry in ALL {
            assert!(!seen.contains(&entry.id), "duplicate provider {}", entry.id);
            seen.push(entry.id);
            assert_eq!(find(entry.id).map(|e| e.id), Some(entry.id));
        }
        assert!(find("nope").is_none());
    }

    /// The order of the table is the autoselect order, so it is a user-visible contract.
    #[test]
    fn autoselect_order_is_openrouter_then_anthropic_then_openai() {
        assert_eq!(
            key_vars(),
            ["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY"]
        );
    }

    #[test]
    fn autoselect_takes_the_first_variable_that_is_set() {
        fn only(want: &'static str) -> impl Fn(&str) -> Option<String> {
            move |var: &str| (var == want).then(|| "k".to_string())
        }
        assert_eq!(
            autoselect(only("OPENAI_API_KEY")).map(|e| e.id),
            Some("openai")
        );
        assert_eq!(
            autoselect(only("ANTHROPIC_API_KEY")).map(|e| e.id),
            Some("anthropic")
        );

        // Both set: the earlier entry wins.
        let two = |var: &str| {
            matches!(var, "ANTHROPIC_API_KEY" | "OPENAI_API_KEY").then(|| "k".to_string())
        };
        assert_eq!(autoselect(two).map(|e| e.id), Some("anthropic"));
    }

    #[test]
    fn a_blank_variable_does_not_count_as_set() {
        let blank = |_: &str| Some("   ".to_string());
        assert!(autoselect(blank).is_none());
    }

    #[test]
    fn nothing_set_selects_nothing_rather_than_a_local_server() {
        assert!(autoselect(|_| None).is_none());
    }

    /// A remote provider without a key variable would fail with a confusing 401 instead of a
    /// clear message; a local one must not demand a key it has no use for.
    #[test]
    fn remote_entries_name_a_key_and_local_ones_do_not() {
        for entry in ALL {
            let local = entry.base_url.contains("127.0.0.1");
            assert_eq!(entry.key_env.is_none(), local, "{} key/locality", entry.id);
            assert!(entry.base_url.ends_with("/v1"), "{} base url", entry.id);
        }
    }
}
