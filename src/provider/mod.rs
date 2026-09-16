//! Three wire formats behind one set of provider-independent types.
//!
//! The types below are the internal model. No dialect's wire shape leaks into them, which is the
//! change that made a second and third format possible: `Message` used to *be* the Chat request
//! body, so there was nothing to translate from.

pub mod anthropic;
pub mod chat;
pub mod http;
pub mod mock;
pub mod registry;
pub mod responses;

use std::pin::Pin;
use std::time::Duration;

use futures_util::Stream;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    /// What the result must be addressed to. Chat and Anthropic use their own id; Responses
    /// distinguishes the streaming item id from the `call_id` a result quotes, and this is the
    /// latter.
    pub id: String,
    pub name: String,
    /// A JSON object as a string. Every dialect streams it in fragments, so it is parsed only
    /// once the turn has assembled it.
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self::text(Role::System, text)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::text(Role::User, text)
    }

    pub fn assistant(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            tool_calls,
            tool_call_id: None,
        }
    }

    pub fn tool_result(call_id: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(body.into()),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.into()),
        }
    }

    fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(text.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    /// Anthropic and Responses report input and output separately and no total.
    pub fn from_parts(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Text(String),
    /// Tool calls arrive interleaved, so fragments carry the key their dialect groups by: the
    /// call index for Chat, the output item id for Responses, the content block index for
    /// Anthropic. `id` and `name` land on the opening fragment only.
    ToolCallDelta {
        key: String,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    Usage(Usage),
    Done,
}

pub type EventStream = Pin<Box<dyn Stream<Item = Result<Event, Error>> + Send>>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("rate limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("provider returned HTTP {status}")]
    Server { status: u16 },
    #[error("request exceeds the model's context window")]
    ContextExceeded,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Error::RateLimited { .. } | Error::Server { .. })
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimited { retry_after } => *retry_after,
            _ => None,
        }
    }
}

/// Which wire format a provider speaks. Fixed per registry entry; `--base-url` moves the endpoint
/// but never the dialect, because a different shape is a different provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Chat,
    Responses,
    Messages,
}

impl Dialect {
    pub fn path(self) -> &'static str {
        match self {
            Dialect::Chat => "chat/completions",
            Dialect::Responses => "responses",
            Dialect::Messages => "messages",
        }
    }

    pub fn build_body(
        self,
        cfg: &Config,
        messages: &[Message],
        tools: &[serde_json::Value],
        cache_key: &str,
    ) -> serde_json::Value {
        match self {
            Dialect::Chat => chat::build_body(cfg, messages, tools, cache_key),
            Dialect::Responses => responses::build_body(cfg, messages, tools, cache_key),
            Dialect::Messages => anthropic::build_body(cfg, messages, tools),
        }
    }

    /// One SSE frame in, zero or more provider-independent events out. Only Anthropic needs the
    /// event name; the OpenAI dialects carry their discriminator inside the data.
    pub fn parse_frame(self, event: &str, data: &str) -> Vec<Result<Event, Error>> {
        match self {
            Dialect::Chat => chat::parse_frame(data),
            Dialect::Responses => responses::parse_frame(data),
            Dialect::Messages => anthropic::parse_frame(event, data),
        }
    }
}

pub enum Provider {
    Http(http::Http),
    Mock(mock::Mock),
}

impl Provider {
    pub async fn stream(
        &self,
        cfg: &Config,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<EventStream, Error> {
        match self {
            Provider::Http(p) => p.stream(cfg, messages, tools).await,
            Provider::Mock(p) => p.stream().await,
        }
    }
}
