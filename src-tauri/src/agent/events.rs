use serde::Serialize;
use serde_json::Value;

/// Streamed to the frontend over a Tauri channel while a request runs.
///
/// Mirrored in `src/lib/ipc.ts` (`AgentEvent`).
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AgentEvent {
    Token {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        title: String,
        args: Value,
        /// Triggered directly by a phrase, without the model.
        fast: bool,
    },
    ToolAwaitingConfirmation {
        call_id: String,
        preview: String,
    },
    ToolResult {
        call_id: String,
        ok: bool,
        output: String,
        duration_ms: u64,
        /// Secrets the tool needs but that are not stored yet (only reported on failure).
        missing_secrets: Vec<String>,
    },
    Done,
    Cancelled,
    Error {
        message: String,
    },
}
