pub mod auth;
pub mod chat;
pub mod models;
pub mod providers;
pub mod sidecar;
pub mod storage;

pub use models::{Conversation, Message, Model};
pub use providers::{AnthropicProvider, GoogleProvider, LLMProvider, OpenAIProvider, Provider};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    storage::run().expect("failed to run BuilderBoard");
}
