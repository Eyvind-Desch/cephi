//! The agent loop: model → tool calls → results → model, until the model answers.

mod events;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use rust_i18n::t;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::llm::{ChatMessage, LlmClient, LlmError, LlmRequest, Role, ToolCall, LOCAL_PROVIDER_ID};
use crate::mcp::McpTool;
use crate::services;
use crate::state::AppState;
use crate::tools::{self, select, ExecContext, Permission, ToolOutput};

pub use events::AgentEvent;

/// Owns the conversation and coordinates the active run.
#[derive(Default)]
pub struct AgentController {
    history: tokio::sync::Mutex<Vec<ChatMessage>>,
    active_run: Mutex<Option<CancellationToken>>,
    pending_confirmations: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

enum Outcome {
    Done,
    Cancelled,
}

impl AgentController {
    pub fn cancel(&self) {
        if let Some(token) = self.active_run.lock().expect("run lock poisoned").as_ref() {
            token.cancel();
        }
    }

    /// Resolves a pending confirmation. Returns `false` if it no longer exists.
    pub fn confirm(&self, call_id: &str, approved: bool) -> bool {
        let sender = self
            .pending_confirmations
            .lock()
            .expect("confirmation lock poisoned")
            .remove(call_id);
        sender.is_some_and(|tx| tx.send(approved).is_ok())
    }

    pub async fn reset(&self) {
        self.cancel();
        self.history.lock().await.clear();
    }

    /// Continues a saved conversation (at startup, or an archived chat after `reset`).
    pub fn restore(&self, history: Vec<ChatMessage>) {
        match self.history.try_lock() {
            Ok(mut current) => *current = history,
            Err(_) => log::warn!("chat not restored: a request is already running"),
        }
    }

    /// The conversation as the model sees it. Waits for a running request to finish.
    pub async fn history(&self) -> Vec<ChatMessage> {
        self.history.lock().await.clone()
    }

    fn begin_run(&self) -> AppResult<CancellationToken> {
        let mut active = self.active_run.lock().expect("run lock poisoned");
        if active.is_some() {
            return Err(AppError::msg(t!("agent.busy")));
        }
        let token = CancellationToken::new();
        *active = Some(token.clone());
        Ok(token)
    }

    fn end_run(&self) {
        *self.active_run.lock().expect("run lock poisoned") = None;
        self.pending_confirmations
            .lock()
            .expect("confirmation lock poisoned")
            .clear();
    }
}

/// What starts a run.
pub enum Input {
    /// A new user message, optionally about pictures (PNG, base64), e.g. a screenshot.
    Message { text: String, images: Vec<String> },
    /// Let the model answer from the tool results at the end of the history
    /// (e.g. "summarize" on search results that were only shown).
    Continue,
}

/// Handles one user message. Progress is reported through `events`; the future resolves
/// when the run is finished, cancelled or failed.
pub async fn run(
    app: &AppHandle,
    state: &AppState,
    input: Input,
    profile_id: Option<String>,
    events: Channel<AgentEvent>,
) -> AppResult<()> {
    let cancel = state.agent.begin_run()?;
    let mut history = state.agent.history.lock().await;
    let rollback_len = history.len();
    let config = state.config();

    let run = Run {
        app,
        state,
        config: &config,
        cancel: &cancel,
        events: &events,
    };
    let result = run.execute(&mut history, input, profile_id.as_deref()).await;

    let event = match result {
        Ok(Outcome::Done) => AgentEvent::Done,
        Ok(Outcome::Cancelled) => AgentEvent::Cancelled,
        Err(message) => {
            log::error!("agent run failed: {message}");
            if history.len() == rollback_len + 1 {
                // Nothing was answered: drop the user message so the history stays consistent.
                history.truncate(rollback_len);
            }
            AgentEvent::Error { message }
        }
    };
    let _ = events.send(event);

    drop(history);
    state.agent.end_run();
    Ok(())
}

struct Run<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
    config: &'a Config,
    cancel: &'a CancellationToken,
    events: &'a Channel<AgentEvent>,
}

impl Run<'_> {
    fn emit(&self, event: AgentEvent) {
        let _ = self.events.send(event);
    }

    async fn execute(
        &self,
        history: &mut Vec<ChatMessage>,
        input: Input,
        profile_id: Option<&str>,
    ) -> Result<Outcome, String> {
        let settings = &self.config.settings;
        let profile = settings.profile(profile_id);
        // The answer after a fast path is a bonus: without a model the result still stands.
        let mut answer_is_optional = false;

        match input {
            Input::Message { text, images } if !images.is_empty() => {
                // A question about a picture: no fast path, and the model has to be able to see.
                if !profile.sees_images() {
                    return Err(t!("capture.no_vision", model = profile.model).into());
                }
                history.push(ChatMessage::user_with_images(text, images));
            }
            Input::Message { text, .. } => {
                // Fast path: a trigger phrase runs its tool directly, without waiting for the model.
                // Afterwards the model only gets involved if the tool asks for an answer from its result.
                if let Some(call) = self.fast_path_call(&text) {
                    history.push(ChatMessage::user(text));
                    history.push(ChatMessage::assistant_tool_calls(vec![call.clone()]));
                    let output = self.run_tool_call(&call, true).await;
                    let answer = output.ok
                        && self
                            .config
                            .tools
                            .get(&call.name)
                            .is_some_and(|tool| tool.answers_after_trigger(settings));
                    history.push(ChatMessage::tool_result(&call, output.content, output.ok));
                    if !answer || self.cancel.is_cancelled() {
                        return Ok(Outcome::Done);
                    }
                    answer_is_optional = true;
                } else {
                    history.push(ChatMessage::user(text));
                }
            }
            Input::Continue => {
                if history.last().is_none_or(|m| m.role != Role::Tool) {
                    return Err(t!("agent.nothing_to_continue").into());
                }
            }
        }

        let client = match LlmClient::for_profile(settings, profile, &self.state.http) {
            Ok(client) => client,
            Err(error) if answer_is_optional => return Ok(self.skip_answer(&error)),
            Err(error) => return Err(error.to_string()),
        };
        let tool_definitions = if profile.tools {
            // Cephi's own tools and the MCP tools share one budget. A local model reads every
            // definition before it answers (~1500 tokens for all of them), so it only gets the ones
            // that fit the request; a cloud model gets everything (see `tools::select`).
            let mut offered = self.config.tools.offered();
            offered.extend(self.state.mcp.offered());
            let limit = if profile.provider == LOCAL_PROVIDER_ID {
                select::LOCAL_LIMIT
            } else {
                select::CLOUD_LIMIT
            };
            let chosen = select::relevant(
                offered,
                |(definition, context)| select::Candidate {
                    name: &definition.name,
                    context,
                    description: &definition.description,
                },
                &recent_requests(history),
                &used_tools(history),
                limit,
            );
            chosen.into_iter().map(|(definition, _)| definition).collect()
        } else {
            Vec::new()
        };
        let system_prompt = self.system_prompt();

        for _ in 0..settings.agent.max_tool_rounds {
            let request = LlmRequest {
                system: &system_prompt,
                messages: history,
                tools: &tool_definitions,
                profile,
            };

            let reply = match client
                .chat(&request, self.cancel, |token| {
                    self.emit(AgentEvent::Token { text: token.into() })
                })
                .await
            {
                Ok(reply) => reply,
                Err(LlmError::Cancelled { partial }) => {
                    if !partial.is_empty() {
                        history.push(ChatMessage::assistant(partial));
                    }
                    return Ok(Outcome::Cancelled);
                }
                Err(error @ (LlmError::Unreachable { .. } | LlmError::Auth { .. })) if answer_is_optional => {
                    return Ok(self.skip_answer(&error));
                }
                Err(error) => return Err(error.to_string()),
            };
            answer_is_optional = false;

            let calls = reply.tool_calls.clone();
            history.push(reply);
            if calls.is_empty() {
                return Ok(Outcome::Done);
            }

            for call in calls {
                if self.cancel.is_cancelled() {
                    return Ok(Outcome::Cancelled);
                }
                let output = self.run_tool_call(&call, false).await;
                history.push(ChatMessage::tool_result(&call, output.content, output.ok));
            }
        }

        let rounds = settings.agent.max_tool_rounds;
        Err(t!("agent.too_many_rounds", rounds = rounds).into())
    }

    /// The model is not available for an optional answer: keep the tool result, explain quietly.
    fn skip_answer(&self, error: &LlmError) -> Outcome {
        log::info!("optional answer skipped: {error}");
        self.emit(AgentEvent::Notice {
            message: t!("agent.answer_unavailable", reason = error.to_string()).into(),
        });
        Outcome::Done
    }

    /// The first trigger match whose arguments are valid and whose tool accepts the fast path.
    fn fast_path_call(&self, text: &str) -> Option<ToolCall> {
        let ctx = self.exec_context();
        self.config
            .tools
            .trigger_matches(text)
            .find(|(tool, raw_args)| {
                tool.normalize_args(raw_args)
                    .is_ok_and(|args| tools::accepts_fast_path(&ctx, tool, &args))
            })
            .map(|(tool, raw_args)| {
                log::info!("fast path: {}", tool.manifest.name);
                ToolCall {
                    id: format!("fast_{}", uuid::Uuid::new_v4().simple()),
                    name: tool.manifest.name.clone(),
                    arguments: raw_args,
                }
            })
    }

    fn exec_context(&self) -> ExecContext<'_> {
        ExecContext {
            app: self.app,
            http: &self.state.http,
        }
    }

    async fn run_tool_call(&self, call: &ToolCall, fast: bool) -> ToolOutput {
        let settings = &self.config.settings.agent;
        let call_id = uuid::Uuid::new_v4().to_string();
        let name = &call.name;
        let tool = self.config.tools.get(name);
        let mcp_tool = if tool.is_none() {
            self.state.mcp.find(name)
        } else {
            None
        };

        self.emit(AgentEvent::ToolCall {
            call_id: call_id.clone(),
            name: name.clone(),
            title: match (tool, &mcp_tool) {
                (Some(tool), _) => tool.title(),
                (None, Some(mcp)) => format!("{} · {}", mcp.title, mcp.server),
                (None, None) => name.clone(),
            },
            args: call.arguments.clone(),
            fast,
        });

        let started = Instant::now();
        let output = match tool {
            None => match &mcp_tool {
                Some(mcp) => self.run_mcp_tool(&call_id, mcp, &call.arguments).await,
                None => ToolOutput::error(t!("agent.unknown_tool", name = name)),
            },
            Some(tool) => match tool.normalize_args(&call.arguments) {
                Err(message) => ToolOutput::error(message),
                Ok(args) => {
                    let approved = tool.manifest.permission == Permission::Auto
                        || self.await_confirmation(&call_id, tools::preview(tool, &args)).await;
                    if approved {
                        tools::execute(&self.exec_context(), tool, &args).await
                    } else {
                        ToolOutput::error(t!("agent.denied"))
                    }
                }
            },
        };

        let content = truncate(output.content, settings.max_tool_output_chars);
        let (missing_secrets, stopped_services) = match tool {
            // Only on failure: tell the UI what is missing, so it can offer keys or a start button.
            Some(tool) if !output.ok => (
                tool.manifest
                    .secrets()
                    .into_iter()
                    .filter(|name| !crate::secrets::is_set(name))
                    .collect(),
                services::stopped(&self.state.http, &self.config.settings, tool.manifest.services()).await,
            ),
            _ => (Vec::new(), Vec::new()),
        };
        self.emit(AgentEvent::ToolResult {
            call_id,
            ok: output.ok,
            output: content.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            missing_secrets,
            stopped_services,
            display: output.display,
        });
        ToolOutput {
            ok: output.ok,
            content,
            display: None,
        }
    }

    /// A tool from an MCP server: asks first unless approved in `mcp.json`, then forwards the call.
    async fn run_mcp_tool(&self, call_id: &str, tool: &McpTool, arguments: &serde_json::Value) -> ToolOutput {
        if !tool.auto_approved {
            let preview = t!(
                "agent.mcp_preview",
                server = tool.server,
                tool = tool.name,
                arguments = serde_json::to_string_pretty(arguments).unwrap_or_default()
            );
            if !self.await_confirmation(call_id, preview.into()).await {
                return ToolOutput::error(t!("agent.denied"));
            }
        }
        match self.state.mcp.call(tool, arguments.clone()).await {
            Ok(content) => ToolOutput {
                ok: true,
                content,
                display: None,
            },
            Err(message) => ToolOutput::error(message),
        }
    }

    async fn await_confirmation(&self, call_id: &str, preview: String) -> bool {
        let (tx, rx) = oneshot::channel();
        self.state
            .agent
            .pending_confirmations
            .lock()
            .expect("confirmation lock poisoned")
            .insert(call_id.to_string(), tx);

        self.emit(AgentEvent::ToolAwaitingConfirmation {
            call_id: call_id.to_string(),
            preview,
        });

        let timeout = Duration::from_secs(self.config.settings.agent.confirm_timeout_secs);
        let approved = tokio::select! {
            answer = rx => answer.unwrap_or(false),
            _ = self.cancel.cancelled() => false,
            _ = tokio::time::sleep(timeout) => false,
        };

        self.state
            .agent
            .pending_confirmations
            .lock()
            .expect("confirmation lock poisoned")
            .remove(call_id);
        approved
    }

    fn system_prompt(&self) -> String {
        // Model-facing, so English like the default prompt.
        let now = chrono::Local::now().format("%A, %Y-%m-%d %H:%M");
        // Short inputs like "? serendipity" don't reveal the user's language; the app language does.
        let language = match rust_i18n::locale().as_ref() {
            "de" => "German",
            _ => "English",
        };
        format!(
            "{}\n\nCurrent date and time: {now}.\nThe user's language is {language}: answer in {language} \
             unless the user writes full sentences in another language.",
            self.config.settings.agent.system_prompt
        )
    }
}

fn truncate(mut text: String, max_chars: usize) -> String {
    if let Some((index, _)) = text.char_indices().nth(max_chars) {
        text.truncate(index);
        text.push('\n');
        text.push_str(&t!("agent.truncated"));
    }
    text
}

/// The last two user messages: a follow-up ("and in Notion?") still needs the words of the first.
fn recent_requests(history: &[ChatMessage]) -> String {
    let mut texts: Vec<&str> = history
        .iter()
        .rev()
        .filter(|m| m.role == Role::User)
        .take(2)
        .map(|m| m.content.as_str())
        .collect();
    texts.reverse();
    texts.join("\n")
}

/// Tools called earlier in this chat.
fn used_tools(history: &[ChatMessage]) -> HashSet<String> {
    history
        .iter()
        .flat_map(|m| m.tool_calls.iter().map(|call| call.name.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundary() {
        // Multi-byte characters on purpose.
        assert_eq!(truncate("äöü".into(), 10), "äöü");
        assert_eq!(truncate("äöüß".into(), 2), format!("äö\n{}", t!("agent.truncated")));
    }
}
