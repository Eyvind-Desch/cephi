//! OpenAI Chat Completions and compatible APIs (Gemini's OpenAI endpoint, OpenRouter, Groq,
//! LM Studio, …) with SSE streaming.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use tokio_util::sync::CancellationToken;

use super::sse::{self, Flow, ReadOutcome};
use super::{
    data_url, new_call_id, parse_arguments, status_error, transport_error, ChatMessage, LlmError, LlmRequest,
    NativeTurn, Role, ToolCall, ToolDef,
};
use crate::config::ProviderSettings;

pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    provider_id: String,
    name: String,
    max_tokens: Option<u32>,
}

impl OpenAiClient {
    pub fn new(http: reqwest::Client, provider: &ProviderSettings, api_key: String) -> Self {
        Self {
            http,
            // Validated in settings: `openai` providers always have a base URL.
            base_url: provider
                .base_url
                .as_deref()
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_string(),
            api_key,
            provider_id: provider.id.clone(),
            name: provider.display_name(),
            max_tokens: provider.max_tokens,
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
            "messages": self.wire_messages(request),
            "stream": true,
        });
        if !request.tools.is_empty() {
            body["tools"] = wire_tools(request.tools);
        }
        if let Some(temperature) = profile.temperature {
            body["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = self.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        let send = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send();
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(LlmError::Cancelled { partial: String::new() }),
            response = send => response.map_err(|e| transport_error(&self.name, e))?,
        };
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(status_error(&self.name, status, &body));
        }

        let mut text = String::new();
        // Raw tool call objects by stream index, kept complete (incl. provider extras such as
        // Gemini thought signatures) so they can be replayed unchanged.
        let mut calls: BTreeMap<u64, Map<String, Value>> = BTreeMap::new();
        let mut finish_reason = String::new();

        let outcome = sse::read(
            response,
            cancel,
            |data| {
                if data.trim() == "[DONE]" {
                    return Ok(Flow::Stop);
                }
                let chunk: Value = serde_json::from_str(data).map_err(|e| LlmError::Protocol(e.to_string()))?;
                if !chunk["error"].is_null() {
                    return Err(LlmError::Api {
                        provider: self.name.clone(),
                        details: chunk["error"]["message"].as_str().unwrap_or(data).to_string(),
                    });
                }
                let choice = &chunk["choices"][0];
                let delta = &choice["delta"];
                if let Some(content) = delta["content"].as_str() {
                    if !content.is_empty() {
                        on_token(content);
                        text.push_str(content);
                    }
                }
                if let Some(deltas) = delta["tool_calls"].as_array() {
                    for part in deltas {
                        let index = part["index"].as_u64().unwrap_or(calls.len() as u64);
                        merge_tool_call_delta(calls.entry(index).or_default(), part);
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    finish_reason = reason.to_string();
                }
                Ok(Flow::Continue)
            },
            |e| transport_error(&self.name, e),
        )
        .await?;

        if matches!(outcome, ReadOutcome::Cancelled) {
            return Err(LlmError::Cancelled { partial: text });
        }

        let mut reply = ChatMessage::assistant(text);
        let mut native_calls = Vec::new();
        for mut raw in calls.into_values() {
            let id = match raw.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
                Some(id) => id.to_string(),
                None => {
                    let id = new_call_id();
                    raw.insert("id".into(), json!(id));
                    id
                }
            };
            raw.entry("type").or_insert_with(|| json!("function"));
            let function = raw.get("function").cloned().unwrap_or_default();
            reply.tool_calls.push(ToolCall {
                id,
                name: function["name"].as_str().unwrap_or_default().to_string(),
                arguments: parse_arguments(function["arguments"].as_str().unwrap_or_default()),
            });
            native_calls.push(Value::Object(raw));
        }

        match finish_reason.as_str() {
            "length" if !reply.tool_calls.is_empty() => return Err(LlmError::Truncated),
            "content_filter" if reply.content.trim().is_empty() && reply.tool_calls.is_empty() => {
                return Err(LlmError::Refused)
            }
            _ => {}
        }

        if !native_calls.is_empty() {
            reply.native = Some(NativeTurn {
                provider: self.provider_id.clone(),
                model: request.profile.model.clone(),
                content: Value::Array(native_calls),
            });
        }
        Ok(reply)
    }

    pub async fn check(&self) -> Result<(), LlmError> {
        let response = self
            .http
            .get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
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
        let mut wire = vec![json!({ "role": "system", "content": request.system })];
        for message in request.messages {
            match message.role {
                // With pictures the content becomes a list of parts (data URLs).
                Role::User if !message.images.is_empty() => {
                    let mut parts: Vec<Value> = message
                        .images
                        .iter()
                        .map(|image| json!({ "type": "image_url", "image_url": { "url": data_url(image) } }))
                        .collect();
                    parts.push(json!({ "type": "text", "text": message.content }));
                    wire.push(json!({ "role": "user", "content": parts }));
                }
                Role::User => wire.push(json!({ "role": "user", "content": message.content })),
                Role::Assistant => {
                    if message.content.trim().is_empty() && message.tool_calls.is_empty() {
                        continue;
                    }
                    let mut entry = json!({ "role": "assistant", "content": message.content });
                    if !message.tool_calls.is_empty() {
                        entry["tool_calls"] = match &message.native {
                            Some(native)
                                if native.provider == self.provider_id && native.model == request.profile.model =>
                            {
                                native.content.clone()
                            }
                            _ => message
                                .tool_calls
                                .iter()
                                .map(|call| {
                                    json!({
                                        "id": call.id,
                                        "type": "function",
                                        "function": { "name": call.name, "arguments": call.arguments.to_string() },
                                    })
                                })
                                .collect(),
                        };
                    }
                    wire.push(entry);
                }
                Role::Tool => wire.push(json!({
                    "role": "tool",
                    "tool_call_id": message.tool_call_id,
                    "content": message.content,
                })),
            }
        }
        wire
    }
}

/// Streamed tool calls arrive in pieces: strings are appended (`function.arguments`), everything
/// else (id, name, provider extras) is taken as sent.
fn merge_tool_call_delta(target: &mut Map<String, Value>, delta: &Value) {
    let Some(object) = delta.as_object() else { return };
    for (key, value) in object {
        match (key.as_str(), value) {
            ("index", _) => {}
            ("function", Value::Object(function)) => {
                let entry = target.entry("function").or_insert_with(|| json!({}));
                for (field, part) in function {
                    match (field.as_str(), part.as_str()) {
                        ("arguments", Some(piece)) => {
                            let existing = entry[field].as_str().unwrap_or_default().to_string();
                            entry[field] = json!(existing + piece);
                        }
                        _ => entry[field] = part.clone(),
                    }
                }
            }
            (_, Value::Null) => {}
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Keeps only widely supported JSON schema keywords (some compatible APIs reject e.g. `default`).
fn wire_tools(tools: &[ToolDef]) -> Value {
    tools
        .iter()
        .map(|tool| {
            let mut parameters = json!({ "type": "object", "properties": {} });
            if let Some(properties) = tool.parameters["properties"].as_object() {
                for (name, spec) in properties {
                    let mut clean = json!({ "type": spec["type"] });
                    for keyword in ["description", "enum"] {
                        if !spec[keyword].is_null() {
                            clean[keyword] = spec[keyword].clone();
                        }
                    }
                    parameters["properties"][name] = clean;
                }
            }
            if let Some(required) = tool.parameters["required"].as_array().filter(|r| !r.is_empty()) {
                parameters["required"] = Value::Array(required.clone());
            }
            json!({
                "type": "function",
                "function": { "name": tool.name, "description": tool.description, "parameters": parameters },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_streamed_tool_call_pieces_and_keeps_extras() {
        let mut call = Map::new();
        merge_tool_call_delta(
            &mut call,
            &json!({ "index": 0, "id": "call_1", "type": "function", "function": { "name": "weather", "arguments": "{\"loc" } }),
        );
        merge_tool_call_delta(
            &mut call,
            &json!({ "index": 0, "function": { "arguments": "ation\":\"Berlin\"}" }, "extra_content": { "google": { "thought_signature": "sig" } } }),
        );
        assert_eq!(call["function"]["arguments"], "{\"location\":\"Berlin\"}");
        assert_eq!(call["extra_content"]["google"]["thought_signature"], "sig");
        assert!(!call.contains_key("index"));
    }

    #[test]
    fn strips_unsupported_schema_keywords() {
        let tools = [ToolDef {
            name: "t".into(),
            description: "d".into(),
            parameters: json!({
                "type": "object",
                "properties": { "q": { "type": "string", "description": "x", "default": "y", "enum": ["y"] } },
                "required": []
            }),
        }];
        let wire = wire_tools(&tools);
        let q = &wire[0]["function"]["parameters"]["properties"]["q"];
        assert!(q["default"].is_null());
        assert_eq!(q["enum"][0], "y");
        assert!(wire[0]["function"]["parameters"]["required"].is_null());
    }

    #[test]
    fn a_screenshot_becomes_a_data_url_part() {
        let settings = crate::config::Settings::default();
        let profile = settings.profile(Some("gemini")).clone();
        let provider = ProviderSettings {
            id: "gemini".into(),
            kind: crate::config::ProviderKind::Openai,
            label: None,
            base_url: Some("https://example.invalid/v1".into()),
            api_key_secret: None,
            max_tokens: None,
            fallbacks: false,
        };
        let client = OpenAiClient::new(reqwest::Client::new(), &provider, "key".into());
        let messages = [ChatMessage::user_with_images(
            "What is on the screen?",
            vec!["QUJD".into()],
        )];
        let tools: Vec<ToolDef> = Vec::new();
        let wire = client.wire_messages(&LlmRequest {
            system: "",
            messages: &messages,
            tools: &tools,
            profile: &profile,
        });
        assert_eq!(wire[1]["content"][0]["type"], "image_url");
        assert_eq!(wire[1]["content"][0]["image_url"]["url"], "data:image/png;base64,QUJD");
        assert_eq!(wire[1]["content"][1]["text"], "What is on the screen?");
    }
}
