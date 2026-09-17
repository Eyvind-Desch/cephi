use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FunctionCall {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

impl ChatMessage {
    fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_name: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn tool(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_name: Some(name.into()),
            ..Self::new(Role::Tool, content)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "<[Value]>::is_empty")]
    pub tools: &'a [Value],
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
    pub keep_alive: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ModelOptions>,
}

#[derive(Debug, Serialize)]
pub struct ModelOptions {
    pub temperature: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Ollama ist nicht erreichbar unter {0}. Läuft Ollama?")]
    Unreachable(String),
    #[error("Ollama: {0}")]
    Api(String),
    #[error("Ungültige Antwort von Ollama: {0}")]
    Protocol(String),
    /// Cancelled by the user; carries the text generated so far.
    #[error("Abgebrochen")]
    Cancelled { partial: String },
}

#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    message: Option<ChatMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(http: reqwest::Client, base_url: &str) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Streams a chat completion. `on_token` receives content deltas as they arrive;
    /// the fully assembled assistant message (including tool calls) is returned.
    pub async fn chat(
        &self,
        request: &ChatRequest<'_>,
        cancel: &CancellationToken,
        mut on_token: impl FnMut(&str),
    ) -> Result<ChatMessage, LlmError> {
        let url = format!("{}/api/chat", self.base_url);
        let send = self.http.post(&url).json(request).send();
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(LlmError::Cancelled { partial: String::new() }),
            response = send => response.map_err(|e| self.map_transport_error(e))?,
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ErrorBody>(&body)
                .map(|b| b.error)
                .unwrap_or(body);
            return Err(LlmError::Api(format!("HTTP {status}: {message}")));
        }

        let mut reply = ChatMessage::assistant(String::new());
        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        loop {
            let next = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(LlmError::Cancelled { partial: reply.content }),
                next = stream.next() => next,
            };
            let Some(bytes) = next else { break };
            buffer.extend_from_slice(&bytes.map_err(|e| self.map_transport_error(e))?);

            // The response is NDJSON: one JSON object per line.
            while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=newline).collect();
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                let chunk: StreamChunk =
                    serde_json::from_slice(&line).map_err(|e| LlmError::Protocol(e.to_string()))?;
                if let Some(error) = chunk.error {
                    return Err(LlmError::Api(error));
                }
                if let Some(message) = chunk.message {
                    if !message.content.is_empty() {
                        on_token(&message.content);
                        reply.content.push_str(&message.content);
                    }
                    reply.tool_calls.extend(message.tool_calls);
                }
                if chunk.done {
                    return Ok(reply);
                }
            }
        }

        Err(LlmError::Protocol("Stream endete unerwartet.".into()))
    }

    fn map_transport_error(&self, error: reqwest::Error) -> LlmError {
        if error.is_connect() {
            LlmError::Unreachable(self.base_url.clone())
        } else {
            LlmError::Api(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_request_without_empty_fields() {
        let messages = [ChatMessage::user("hi")];
        let request = ChatRequest {
            model: "m",
            messages: &messages,
            tools: &[],
            stream: true,
            think: None,
            keep_alive: "5m",
            options: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "model": "m",
                "messages": [{ "role": "user", "content": "hi" }],
                "stream": true,
                "keep_alive": "5m"
            })
        );
    }

    #[test]
    fn parses_tool_call_chunk() {
        let chunk: StreamChunk = serde_json::from_str(
            r#"{"model":"m","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"weather","arguments":{"location":"Berlin"}}}]},"done":false}"#,
        )
        .unwrap();
        let message = chunk.message.unwrap();
        assert_eq!(message.tool_calls[0].function.name, "weather");
        assert_eq!(message.tool_calls[0].function.arguments["location"], "Berlin");
    }
}
