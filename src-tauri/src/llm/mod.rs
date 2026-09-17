//! Language model access. Currently Ollama only.

mod ollama;

pub use ollama::{ChatMessage, ChatRequest, LlmError, ModelOptions, OllamaClient, ToolCall};
