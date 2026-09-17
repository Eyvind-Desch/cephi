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
    },
    ToolAwaitingConfirmation {
        call_id: String,
        preview: String,
    },
    ToolResult {
        call_id: String,
        ok: bool,
        output: String,
    },
    Done,
    Cancelled,
    Error {
        message: String,
    },
}
