//! Language model access. Currently Ollama only.

mod ollama;

pub use ollama::{ChatMessage, ChatRequest, FunctionCall, LlmError, ModelOptions, OllamaClient, ToolCall};
