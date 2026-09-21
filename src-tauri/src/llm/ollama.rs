//! Local models via Ollama's `/api/chat` (NDJSON streaming).

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::{
    new_call_id, status_error, transport_error, ChatMessage, LlmError, LlmRequest, Role, ToolCall, ToolDef,
    LOCAL_PROVIDER_ID,
};
use crate::config::OllamaSettings;

pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
    keep_alive: String,
}

#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    message: Option<WireMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct WireMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<WireToolCall>,
}

#[derive(Deserialize)]
struct WireToolCall {
    #[serde(default)]
    id: Option<String>,
    function: WireFunction,
}

#[derive(Deserialize)]
struct WireFunction {
    name: String,
    #[serde(default)]
    arguments: Value,
}

impl OllamaClient {
    pub fn new(http: reqwest::Client, settings: &OllamaSettings) -> Self {
        Self {
            http,
            base_url: settings.base_url.trim_end_matches('/').to_string(),
            keep_alive: settings.keep_alive.clone(),
        }
    }

    pub async fn chat(
        &self,
        request: &LlmRequest<'_>,
        cancel: &CancellationToken,
        mut on_token: impl FnMut(&str),
    ) -> Result<ChatMessage, LlmError> {
        let profile = request.profile;
        let mut body = json!({
            "model": profile.model,
            "messages": wire_messages(request),
            "stream": true,
            "keep_alive": self.keep_alive,
        });
        if !request.tools.is_empty() {
            body["tools"] = wire_tools(request.tools);
        }
        if let Some(think) = profile.think {
            body["think"] = json!(think);
        }
        if let Some(temperature) = profile.temperature {
            body["options"] = json!({ "temperature": temperature });
        }

        let send = self.http.post(format!("{}/api/chat", self.base_url)).json(&body).send();
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(LlmError::Cancelled { partial: String::new() }),
            response = send => response.map_err(|e| self.transport(e))?,
        };
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(status_error(LOCAL_PROVIDER_ID, status, &body));
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
            buffer.extend_from_slice(&bytes.map_err(|e| self.transport(e))?);

            // One JSON object per line.
            while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=newline).collect();
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                let chunk: StreamChunk =
                    serde_json::from_slice(&line).map_err(|e| LlmError::Protocol(e.to_string()))?;
                if let Some(error) = chunk.error {
                    return Err(LlmError::Api {
                        provider: LOCAL_PROVIDER_ID.into(),
                        details: error,
                    });
                }
                if let Some(message) = chunk.message {
                    if !message.content.is_empty() {
                        on_token(&message.content);
                        reply.content.push_str(&message.content);
                    }
                    reply
                        .tool_calls
                        .extend(message.tool_calls.into_iter().map(|call| ToolCall {
                            id: call.id.unwrap_or_else(new_call_id),
                            name: call.function.name,
                            arguments: call.function.arguments,
                        }));
                }
                if chunk.done {
                    return Ok(reply);
                }
            }
        }

        Err(LlmError::Protocol("stream ended unexpectedly".into()))
    }

    pub async fn check(&self) -> Result<(), LlmError> {
        let response = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            Err(status_error(
                LOCAL_PROVIDER_ID,
                status,
                &response.text().await.unwrap_or_default(),
            ))
        }
    }

    /// Loads a model into memory without generating anything.
    pub async fn preload(&self, model: &str) -> Result<(), LlmError> {
        let response = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&json!({ "model": model, "keep_alive": self.keep_alive }))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            Err(status_error(
                LOCAL_PROVIDER_ID,
                status,
                &response.text().await.unwrap_or_default(),
            ))
        }
    }

    fn transport(&self, error: reqwest::Error) -> LlmError {
        match transport_error(LOCAL_PROVIDER_ID, error) {
            // The local message names the URL, which helps more than the raw error.
            LlmError::Unreachable { provider, .. } => LlmError::Unreachable {
                provider,
                details: self.base_url.clone(),
            },
            other => other,
        }
    }
}

fn wire_messages(request: &LlmRequest<'_>) -> Vec<Value> {
    let mut messages = vec![json!({ "role": "system", "content": request.system })];
    for message in request.messages {
        messages.push(match message.role {
            // Ollama takes pictures as plain base64 next to the text.
            Role::User if !message.images.is_empty() => {
                json!({ "role": "user", "content": message.content, "images": message.images })
            }
            Role::User => json!({ "role": "user", "content": message.content }),
            Role::Assistant => {
                let mut wire = json!({ "role": "assistant", "content": message.content });
                if !message.tool_calls.is_empty() {
                    wire["tool_calls"] = message
                        .tool_calls
                        .iter()
                        .map(|call| json!({ "function": { "name": call.name, "arguments": call.arguments } }))
                        .collect();
                }
                wire
            }
            Role::Tool => json!({
                "role": "tool",
                "tool_name": message.tool_name,
                "content": message.content,
            }),
        });
    }
    messages
}

fn wire_tools(tools: &[ToolDef]) -> Value {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": { "name": tool.name, "description": tool.description, "parameters": tool.parameters },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end against the local Ollama with a real tool call. Run manually:
    /// `cargo test -- --ignored live_chat_calls_a_tool`
    #[test]
    #[ignore = "needs a running Ollama with assistant-fast:latest"]
    fn live_chat_calls_a_tool() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let settings = crate::config::Settings::default();
        let profile = settings.profile(Some("fast")).clone();
        let client = OllamaClient::new(reqwest::Client::new(), &settings.ollama);
        let tools = [ToolDef {
            name: "weather".into(),
            description: "Returns the current weather for a location.".into(),
            parameters: json!({ "type": "object", "properties": { "location": { "type": "string" } }, "required": ["location"] }),
        }];
        let messages = [ChatMessage::user("What's the weather in Hamburg?")];
        let request = LlmRequest {
            system: "You are a helpful assistant. Use tools when they help.",
            messages: &messages,
            tools: &tools,
            profile: &profile,
        };
        let reply = runtime
            .block_on(client.chat(&request, &CancellationToken::new(), |_| {}))
            .unwrap();
        assert_eq!(reply.tool_calls.len(), 1, "{reply:?}");
        assert_eq!(reply.tool_calls[0].name, "weather");
        assert!(!reply.tool_calls[0].id.is_empty());
    }

    #[test]
    fn parses_tool_call_chunk_and_assigns_ids() {
        let chunk: StreamChunk = serde_json::from_str(
            r#"{"model":"m","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"weather","arguments":{"location":"Berlin"}}}]},"done":false}"#,
        )
        .unwrap();
        let call = &chunk.message.unwrap().tool_calls[0];
        assert_eq!(call.function.name, "weather");
        assert_eq!(call.function.arguments["location"], "Berlin");
        assert!(call.id.is_none());
    }

    #[test]
    fn a_screenshot_travels_next_to_the_question() {
        let settings = crate::config::Settings::default();
        let profile = settings.profile(Some("fast")).clone();
        let messages = [ChatMessage::user_with_images(
            "What is on the screen?",
            vec!["QUJD".into()],
        )];
        let tools: Vec<ToolDef> = Vec::new();
        let wire = wire_messages(&LlmRequest {
            system: "",
            messages: &messages,
            tools: &tools,
            profile: &profile,
        });
        assert_eq!(wire[1]["content"], "What is on the screen?");
        assert_eq!(wire[1]["images"][0], "QUJD");
    }
}
