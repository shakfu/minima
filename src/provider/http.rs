//! The network provider. Owns the client, auth and SSE mechanics; the dialect owns the shapes.
//! Classifies failures but does not retry -- that policy lives in `agent.rs`, which knows whether
//! anyone is watching.

use std::time::Duration;

use anyhow::anyhow;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::RETRY_AFTER;

use super::{Dialect, Error, EventStream, Message};
use crate::config::Config;

/// The version Anthropic requires on every native Messages request.
const ANTHROPIC_VERSION: &str = "2023-06-01";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Per read, not per request, so it bounds silence rather than stream length. Generous, because a
/// reasoning model can send nothing for minutes before its first token.
const READ_TIMEOUT: Duration = Duration::from_secs(300);

/// Without timeouts a stalled stream hangs `-p` for good, and nothing in CI sends Ctrl-C.
pub fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
}

/// Native Messages authenticates by header, not bearer, and versions every request. Shared by the
/// completion and the model list, so a gateway that checks one header sees it on both.
pub fn authorize(
    request: reqwest::RequestBuilder,
    dialect: Dialect,
    key: &str,
) -> reqwest::RequestBuilder {
    match dialect {
        Dialect::Messages => request
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION),
        Dialect::Chat | Dialect::Responses => request.bearer_auth(key),
    }
}

pub struct Http {
    client: reqwest::Client,
    /// Improves the provider's prompt-cache hit rate across the turns of one conversation, since
    /// the whole transcript is resent every turn. One process is one conversation today; session
    /// resume would supply the session id here instead.
    cache_key: String,
}

impl Http {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: client()?,
            cache_key: session_cache_key(),
        })
    }

    pub async fn stream(
        &self,
        cfg: &Config,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<EventStream, Error> {
        let dialect = cfg.dialect;
        let url = format!("{}/{}", cfg.base_url, dialect.path());
        let body = dialect.build_body(cfg, messages, tools, &self.cache_key);

        tracing::debug!(%url, model = %cfg.model, messages = messages.len(), ?dialect, "request");

        let request = authorize(self.client.post(&url).json(&body), dialect, &cfg.api_key);

        let response = request
            .send()
            .await
            .map_err(|e| Error::Other(anyhow!(e).context(format!("POST {url}"))))?;

        let status = response.status();
        if !status.is_success() {
            return Err(classify(status, response).await);
        }

        let events = response
            .bytes_stream()
            .eventsource()
            .map(move |frame| match frame {
                Err(e) => vec![Err(Error::Other(
                    anyhow!(e).context("reading the event stream"),
                ))],
                Ok(frame) => dialect.parse_frame(&frame.event, &frame.data),
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(events))
    }
}

/// A body naming the context window is not a client bug worth a stack trace; it is the one
/// failure the user must be told about in plain words.
async fn classify(status: reqwest::StatusCode, response: reqwest::Response) -> Error {
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs);

    let code = status.as_u16();
    let body = response.text().await.unwrap_or_default();

    if code == 429 {
        return Error::RateLimited { retry_after };
    }
    if status.is_server_error() {
        return Error::Server { status: code };
    }
    if is_context_error(&body) {
        return Error::ContextExceeded;
    }
    Error::Other(anyhow!("{}", body.trim()).context(format!("HTTP {code}")))
}

/// Each dialect words it differently, and none of them uses a distinct status code.
fn is_context_error(body: &str) -> bool {
    const MARKERS: [&str; 4] = [
        "context_length_exceeded",
        "maximum context length",
        "prompt is too long",
        "input length and `max_tokens` exceed",
    ];
    MARKERS.iter().any(|m| body.contains(m))
}

/// Stable for the process, distinct between runs. No `rand` dependency for one identifier.
fn session_cache_key() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("minima-{:x}-{nanos:x}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_errors_are_recognised_for_every_dialect() {
        assert!(is_context_error(r#"{"code":"context_length_exceeded"}"#));
        assert!(is_context_error(
            "This model's maximum context length is 8192"
        ));
        assert!(is_context_error(
            r#"{"message":"prompt is too long: 300000 tokens"}"#
        ));
        assert!(!is_context_error(r#"{"code":"invalid_api_key"}"#));
    }

    #[test]
    fn each_dialect_posts_to_its_own_path() {
        assert_eq!(Dialect::Chat.path(), "chat/completions");
        assert_eq!(Dialect::Responses.path(), "responses");
        assert_eq!(Dialect::Messages.path(), "messages");
    }
}
