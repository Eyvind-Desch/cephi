//! The agent loop: model → tool calls → results → model, until the model answers.

mod events;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use rust_i18n::t;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::llm::{ChatMessage, LlmClient, LlmError, LlmRequest, Role, ToolCall};
use crate::services;
use crate::state::AppState;
use crate::tools::{self, ExecContext, Permission, ToolOutput};

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
    /// A new user message.
    Message(String),
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
            Input::Message(text) => {
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
            self.config.tools.definitions()
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

        self.emit(AgentEvent::ToolCall {
            call_id: call_id.clone(),
            name: name.clone(),
            title: tool.map_or_else(|| name.clone(), |t| t.title()),
            args: call.arguments.clone(),
            fast,
        });

        let started = Instant::now();
        let output = match tool {
            None => ToolOutput::error(t!("agent.unknown_tool", name = name)),
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
