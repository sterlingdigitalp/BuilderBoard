pub mod auth;
pub mod builders;
pub mod chat;
pub mod execution;
pub mod filesystem_intent;
pub mod filesystem_tools;
pub mod models;
pub mod native_tool_trace;
pub mod project_scope_cache;
pub mod projects;
pub mod providers;
pub mod runtime_diagnostics;
pub mod sidecar;
pub mod storage;
pub mod stream_execution;
pub mod stream_persistence;
pub mod stream_write_buffer;

pub use models::{Conversation, Message, Model};
pub use providers::{AnthropicProvider, GoogleProvider, LLMProvider, OpenAIProvider, Provider};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    storage::run().expect("failed to run BuilderBoard");
}
