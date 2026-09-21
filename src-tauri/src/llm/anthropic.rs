//! Claude via the Anthropic Messages API (raw HTTP + SSE; there is no official Rust SDK).

use std::collections::BTreeMap;

use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::sse::{self, Flow, ReadOutcome};
use super::{
    parse_arguments, status_error, transport_error, ChatMessage, LlmError, LlmRequest, NativeTurn, Role, ToolCall,
    IMAGE_MEDIA_TYPE,
};
use crate::config::ProviderSettings;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 64_000;
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    provider_id: String,
    name: String,
    max_tokens: u32,
    fallbacks: bool,
}

/// A content block being assembled from stream events.
#[derive(Default)]
struct PartialBlock {
    start: Value,
    text: String,
    input_json: String,
    thinking: String,
    signature: String,
}

impl AnthropicClient {
    pub fn new(http: reqwest::Client, provider: &ProviderSettings, api_key: String) -> Self {
        Self {
            http,
            base_url: provider
                .base_url
                .as_deref()
                .unwrap_or(DEFAULT_BASE_URL)
                .trim_end_matches('/')
                .to_string(),
            api_key,
            provider_id: provider.id.clone(),
            name: provider.display_name(),
            max_tokens: provider.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            fallbacks: provider.fallbacks,
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
            "max_tokens": self.max_tokens,
            "stream": true,
            "system": request.system,
            "messages": self.wire_messages(request),
        });
        if !request.tools.is_empty() {
            body["tools"] = request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.parameters,
                        // Inputs stream as generated; they are validated before any tool runs.
                        "eager_input_streaming": true,
                    })
                })
                .collect();
        }
        if let Some(effort) = &profile.effort {
            body["output_config"] = json!({ "effort": effort });
        }

        let mut http_request = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION);
        if self.fallbacks && supports_fallbacks(&profile.model) {
            // Re-runs a declined request on Anthropic's recommended fallback model server-side.
            body["fallbacks"] = json!("default");
            http_request = http_request.header("anthropic-beta", FALLBACK_BETA);
        }
        let http_request = http_request.json(&body);

        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(LlmError::Cancelled { partial: String::new() }),
            response = http_request.send() => response.map_err(|e| transport_error(&self.name, e))?,
        };
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(status_error(&self.name, status, &body));
        }

        let mut served_model = profile.model.clone();
        let mut blocks: BTreeMap<u64, PartialBlock> = BTreeMap::new();
        let mut stop_reason = String::new();
        let mut visible_text = String::new();

        let outcome = sse::read(
            response,
            cancel,
            |data| {
                let event: Value = serde_json::from_str(data).map_err(|e| LlmError::Protocol(e.to_string()))?;
                let index = event["index"].as_u64().unwrap_or_default();
                match event["type"].as_str().unwrap_or_default() {
                    "message_start" => {
                        if let Some(model) = event["message"]["model"].as_str() {
                            served_model = model.to_string();
                        }
                    }
                    "content_block_start" => {
                        blocks.insert(
                            index,
                            PartialBlock {
                                start: event["content_block"].clone(),
                                ..Default::default()
                            },
                        );
                    }
                    "content_block_delta" => {
                        let block = blocks.entry(index).or_default();
                        let delta = &event["delta"];
                        match delta["type"].as_str().unwrap_or_default() {
                            "text_delta" => {
                                let text = delta["text"].as_str().unwrap_or_default();
                                on_token(text);
                                visible_text.push_str(text);
                                block.text.push_str(text);
                            }
                            "input_json_delta" => block
                                .input_json
                                .push_str(delta["partial_json"].as_str().unwrap_or_default()),
                            "thinking_delta" => block.thinking.push_str(delta["thinking"].as_str().unwrap_or_default()),
                            "signature_delta" => block
                                .signature
                                .push_str(delta["signature"].as_str().unwrap_or_default()),
                            _ => {}
                        }
                    }
                    "message_delta" => {
                        if let Some(reason) = event["delta"]["stop_reason"].as_str() {
                            stop_reason = reason.to_string();
                        }
                    }
                    "message_stop" => return Ok(Flow::Stop),
                    "error" => {
                        return Err(LlmError::Api {
                            provider: self.name.clone(),
                            details: event["error"]["message"].as_str().unwrap_or(data).to_string(),
                        })
                    }
                    _ => {} // ping, content_block_stop
                }
                Ok(Flow::Continue)
            },
            |e| transport_error(&self.name, e),
        )
        .await?;

        if matches!(outcome, ReadOutcome::Cancelled) {
            return Err(LlmError::Cancelled { partial: visible_text });
        }

        let mut reply = ChatMessage::assistant(visible_text);
        let mut native_blocks = Vec::new();
        for block in blocks.into_values() {
            match block.start["type"].as_str().unwrap_or_default() {
                "text" if !block.text.is_empty() => native_blocks.push(json!({ "type": "text", "text": block.text })),
                "tool_use" => {
                    let arguments = if block.input_json.trim().is_empty() {
                        block.start["input"].clone()
                    } else {
                        parse_arguments(&block.input_json)
                    };
                    let call = ToolCall {
                        id: block.start["id"].as_str().unwrap_or_default().to_string(),
                        name: block.start["name"].as_str().unwrap_or_default().to_string(),
                        arguments,
                    };
                    let input = if call.arguments.is_object() {
                        call.arguments.clone()
                    } else {
                        json!({})
                    };
                    native_blocks.push(json!({ "type": "tool_use", "id": call.id, "name": call.name, "input": input }));
                    reply.tool_calls.push(call);
                }
                // Must be passed back unchanged (with signature) when continuing on the same model.
                "thinking" => native_blocks.push(json!({
                    "type": "thinking",
                    "thinking": block.thinking,
                    "signature": block.signature,
                })),
                "redacted_thinking" => native_blocks.push(block.start),
                // `fallback` markers and unknown block types are not replayed.
                _ => {}
            }
        }

        match stop_reason.as_str() {
            "refusal" => {
                reply.tool_calls.clear();
                if reply.content.trim().is_empty() {
                    return Err(LlmError::Refused);
                }
                return Ok(reply);
            }
            // A cut-off tool call must never run.
            "max_tokens" if !reply.tool_calls.is_empty() => return Err(LlmError::Truncated),
            _ => {}
        }

        reply.native = Some(NativeTurn {
            provider: self.provider_id.clone(),
            model: served_model,
            content: Value::Array(native_blocks),
        });
        Ok(reply)
    }

    pub async fn check(&self) -> Result<(), LlmError> {
        let response = self
            .http
            .get(format!("{}/v1/models", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .send()
            .await
            .map_err(|e| transport_error(&self.name, e))?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            Err(status_error(
                &self.name,
                status,
                &response.text().await.unwrap_or_default(),
            ))
        }
    }

    fn wire_messages(&self, request: &LlmRequest<'_>) -> Vec<Value> {
        let mut wire: Vec<Value> = Vec::new();
        for message in request.messages {
            match message.role {
                // With pictures the content becomes blocks: the picture first, then the question.
                Role::User if !message.images.is_empty() => {
                    let mut blocks: Vec<Value> = message
                        .images
                        .iter()
                        .map(|image| {
                            json!({
                                "type": "image",
                                "source": { "type": "base64", "media_type": IMAGE_MEDIA_TYPE, "data": image },
                            })
                        })
                        .collect();
                    blocks.push(json!({ "type": "text", "text": message.content }));
                    wire.push(json!({ "role": "user", "content": blocks }));
                }
                Role::User => wire.push(json!({ "role": "user", "content": message.content })),
                Role::Assistant => {
                    let content = match &message.native {
                        // Same provider and model: replay verbatim, including thinking blocks.
                        Some(native)
                            if native.provider == self.provider_id && native.model == request.profile.model =>
                        {
                            native.content.clone()
                        }
                        _ => generic_assistant_blocks(message),
                    };
                    if content.as_array().is_some_and(|blocks| !blocks.is_empty()) {
                        wire.push(json!({ "role": "assistant", "content": content }));
                    }
                }
                Role::Tool => {
                    let result = json!({
                        "type": "tool_result",
                        "tool_use_id": message.tool_call_id,
                        "content": message.content,
                        "is_error": message.tool_error,
                    });
                    // All results of one assistant turn belong in a single user message.
                    match wire.last_mut() {
                        Some(last) if last["role"] == "user" && is_tool_results(&last["content"]) => {
                            last["content"].as_array_mut().expect("checked above").push(result);
                        }
                        _ => wire.push(json!({ "role": "user", "content": [result] })),
                    }
                }
            }
        }
        wire
    }
}

fn generic_assistant_blocks(message: &ChatMessage) -> Value {
    let mut blocks = Vec::new();
    if !message.content.trim().is_empty() {
        blocks.push(json!({ "type": "text", "text": message.content }));
    }
    for call in &message.tool_calls {
        let input = if call.arguments.is_object() {
            call.arguments.clone()
        } else {
            json!({})
        };
        blocks.push(json!({ "type": "tool_use", "id": call.id, "name": call.name, "input": input }));
    }
    Value::Array(blocks)
}

fn is_tool_results(content: &Value) -> bool {
    content
        .as_array()
        .is_some_and(|blocks| blocks.iter().all(|b| b["type"] == "tool_result"))
}

/// Server-side refusal fallbacks are offered for the models with safety classifiers that decline.
fn supports_fallbacks(model: &str) -> bool {
    model.starts_with("claude-opus-5") || model.starts_with("claude-fable-5")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Profile, ProviderKind};
    use crate::llm::ToolDef;
    use crate::tools::LocalizedText;

    fn client() -> AnthropicClient {
        let provider = ProviderSettings {
            id: "anthropic".into(),
            kind: ProviderKind::Anthropic,
            label: None,
            base_url: None,
            api_key_secret: None,
            max_tokens: None,
            fallbacks: true,
        };
        AnthropicClient::new(reqwest::Client::new(), &provider, "key".into())
    }

    fn profile(model: &str) -> Profile {
        Profile {
            id: "claude".into(),
            label: LocalizedText::plain("Claude"),
            provider: "anthropic".into(),
            model: model.into(),
            tools: true,
            vision: true,
            think: None,
            temperature: None,
            effort: None,
        }
    }

    #[test]
    fn groups_tool_results_and_replays_native_turns_only_for_the_same_model() {
        let call = ToolCall {
            id: "toolu_1".into(),
            name: "weather".into(),
            arguments: json!({ "location": "Berlin" }),
        };
        let second = ToolCall {
            id: "toolu_2".into(),
            ..call.clone()
        };
        let mut assistant = ChatMessage::assistant_tool_calls(vec![call.clone(), second.clone()]);
        assistant.native = Some(NativeTurn {
            provider: "anthropic".into(),
            model: "claude-opus-5".into(),
            content: json!([{ "type": "thinking", "thinking": "", "signature": "sig" }, { "type": "tool_use", "id": "toolu_1", "name": "weather", "input": {} }]),
        });
        let messages = vec![
            ChatMessage::user("Weather?"),
            assistant,
            ChatMessage::tool_result(&call, "sunny", true),
            ChatMessage::tool_result(&second, "rain", false),
        ];
        let tools: Vec<ToolDef> = Vec::new();

        let same = profile("claude-opus-5");
        let wire = client().wire_messages(&LlmRequest {
            system: "",
            messages: &messages,
            tools: &tools,
            profile: &same,
        });
        assert_eq!(wire.len(), 3, "tool results merged into one user message");
        assert_eq!(wire[1]["content"][0]["type"], "thinking");
        assert_eq!(wire[2]["content"].as_array().unwrap().len(), 2);
        assert_eq!(wire[2]["content"][1]["is_error"], true);

        let other = profile("claude-haiku-4-5");
        let wire = client().wire_messages(&LlmRequest {
            system: "",
            messages: &messages,
            tools: &tools,
            profile: &other,
        });
        assert_eq!(
            wire[1]["content"][0]["type"], "tool_use",
            "thinking dropped for another model"
        );
    }

    #[test]
    fn fallbacks_only_for_supported_models() {
        assert!(supports_fallbacks("claude-opus-5"));
        assert!(supports_fallbacks("claude-fable-5-1"));
        assert!(!supports_fallbacks("claude-haiku-4-5"));
    }

    #[test]
    fn a_screenshot_becomes_an_image_block_before_the_question() {
        let messages = [ChatMessage::user_with_images("What is this error?", vec!["QUJD".into()])];
        let tools: Vec<ToolDef> = Vec::new();
        let profile = profile("claude-opus-5");
        let wire = client().wire_messages(&LlmRequest {
            system: "",
            messages: &messages,
            tools: &tools,
            profile: &profile,
        });
        assert_eq!(wire[0]["content"][0]["type"], "image");
        assert_eq!(wire[0]["content"][0]["source"]["media_type"], "image/png");
        assert_eq!(wire[0]["content"][0]["source"]["data"], "QUJD");
        assert_eq!(wire[0]["content"][1]["text"], "What is this error?");
    }
}
