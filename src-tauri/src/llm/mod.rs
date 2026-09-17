//! Language model providers behind one interface.
//!
//! The agent works with provider-neutral [`ChatMessage`]s; each client converts them to its wire
//! format. Three protocols cover all supported providers:
//!
//! | Protocol | Providers |
//! |---|---|
//! | [`ollama`] | local models |
//! | [`anthropic`] | Claude |
//! | [`openai`] | OpenAI and compatible APIs: Gemini, OpenRouter, Groq, LM Studio, … |

mod anthropic;
mod ollama;
mod openai;
mod sse;

use rust_i18n::t;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::config::{Profile, ProviderKind, Settings};
use crate::secrets;

pub use anthropic::AnthropicClient;
pub use ollama::OllamaClient;
pub use openai::OpenAiClient;

/// Id of the built-in local provider (configured in `settings.ollama`).
pub const LOCAL_PROVIDER_ID: &str = "ollama";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    Tool,
}

/// Provider-neutral chat message.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    /// Assistant only.
    pub tool_calls: Vec<ToolCall>,
    /// Tool results only.
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_error: bool,
    /// The assistant turn exactly as the provider returned it (e.g. Claude thinking blocks with
    /// signatures). Replayed verbatim when talking to the same provider and model again.
    pub native: Option<NativeTurn>,
}

#[derive(Clone, Debug)]
pub struct NativeTurn {
    pub provider: String,
    pub model: String,
    pub content: Value,
}

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Parsed JSON arguments; a `Value::String` holds unparseable raw input.
    pub arguments: Value,
}

/// A tool as offered to the model.
#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON schema object.
    pub parameters: Value,
}

pub struct LlmRequest<'a> {
    pub system: &'a str,
    pub messages: &'a [ChatMessage],
    pub tools: &'a [ToolDef],
    pub profile: &'a Profile,
}

impl ChatMessage {
    fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            tool_name: None,
            tool_error: false,
            native: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            tool_calls,
            ..Self::new(Role::Assistant, String::new())
        }
    }

    pub fn tool_result(call: &ToolCall, content: impl Into<String>, ok: bool) -> Self {
        Self {
            tool_call_id: Some(call.id.clone()),
            tool_name: Some(call.name.clone()),
            tool_error: !ok,
            ..Self::new(Role::Tool, content)
        }
    }
}

#[derive(Debug)]
pub enum LlmError {
    Unreachable {
        provider: String,
        details: String,
    },
    /// Missing or rejected API key.
    Auth {
        provider: String,
    },
    Api {
        provider: String,
        details: String,
    },
    Protocol(String),
    /// The output limit was hit while the model was writing a tool call.
    Truncated,
    /// The model declined the request.
    Refused,
    /// Cancelled by the user; carries the text generated so far.
    Cancelled {
        partial: String,
    },
}

/// User-facing, localized.
impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            LlmError::Unreachable { provider, details } if provider == LOCAL_PROVIDER_ID => {
                t!("llm.unreachable", url = details)
            }
            LlmError::Unreachable { provider, details } => {
                t!("llm.cloud_unreachable", provider = provider, details = details)
            }
            LlmError::Auth { provider } => t!("llm.auth", provider = provider),
            LlmError::Api { provider, details } => t!("llm.api_error", provider = provider, details = details),
            LlmError::Protocol(details) => t!("llm.invalid_response", details = details),
            LlmError::Truncated => t!("llm.truncated"),
            LlmError::Refused => t!("llm.refused"),
            LlmError::Cancelled { .. } => t!("llm.cancelled"),
        };
        f.write_str(&message)
    }
}

impl std::error::Error for LlmError {}

/// A client for one provider, ready to chat.
pub enum LlmClient {
    Ollama(OllamaClient),
    Anthropic(AnthropicClient),
    OpenAi(OpenAiClient),
}

impl LlmClient {
    /// Builds the client for a profile. Fails if the provider is unknown or its key is missing.
    pub fn for_profile(settings: &Settings, profile: &Profile, http: &reqwest::Client) -> Result<Self, LlmError> {
        Self::for_provider(settings, &profile.provider, http)
    }

    pub fn for_provider(settings: &Settings, provider_id: &str, http: &reqwest::Client) -> Result<Self, LlmError> {
        if provider_id == LOCAL_PROVIDER_ID {
            return Ok(Self::Ollama(OllamaClient::new(http.clone(), &settings.ollama)));
        }
        let provider = settings
            .provider(provider_id)
            .ok_or_else(|| LlmError::Protocol(format!("unknown provider \"{provider_id}\"")))?;
        let key = secrets::get(&provider.api_key_secret())
            .ok()
            .flatten()
            .ok_or_else(|| LlmError::Auth {
                provider: provider.display_name(),
            })?;
        Ok(match provider.kind {
            ProviderKind::Anthropic => Self::Anthropic(AnthropicClient::new(http.clone(), provider, key)),
            ProviderKind::Openai => Self::OpenAi(OpenAiClient::new(http.clone(), provider, key)),
        })
    }

    /// Streams a reply. `on_token` receives visible text as it arrives.
    pub async fn chat(
        &self,
        request: &LlmRequest<'_>,
        cancel: &CancellationToken,
        on_token: impl FnMut(&str),
    ) -> Result<ChatMessage, LlmError> {
        match self {
            Self::Ollama(client) => client.chat(request, cancel, on_token).await,
            Self::Anthropic(client) => client.chat(request, cancel, on_token).await,
            Self::OpenAi(client) => client.chat(request, cancel, on_token).await,
        }
    }

    /// Cheap authenticated request (model list) to verify the key and connectivity.
    pub async fn check(&self) -> Result<(), LlmError> {
        match self {
            Self::Ollama(client) => client.check().await,
            Self::Anthropic(client) => client.check().await,
            Self::OpenAi(client) => client.check().await,
        }
    }
}

/// Maps an HTTP error status and body to an [`LlmError`].
fn status_error(provider: &str, status: reqwest::StatusCode, body: &str) -> LlmError {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return LlmError::Auth {
            provider: provider.to_string(),
        };
    }
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|json| {
            json["error"]["message"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    LlmError::Api {
        provider: provider.to_string(),
        details: format!("HTTP {status}: {message}"),
    }
}

fn transport_error(provider: &str, error: reqwest::Error) -> LlmError {
    let unreachable = error.is_connect() || error.is_timeout();
    let details = error.without_url().to_string();
    if unreachable {
        LlmError::Unreachable {
            provider: provider.to_string(),
            details,
        }
    } else {
        LlmError::Api {
            provider: provider.to_string(),
            details,
        }
    }
}

/// Parses streamed tool arguments; unparseable input is kept as a string so validation reports it.
fn parse_arguments(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return Value::Object(Default::default());
    }
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// Tool call ids are required by cloud APIs; local models do not always send one.
fn new_call_id() -> String {
    format!("call_{}", uuid::Uuid::new_v4().simple())
}
