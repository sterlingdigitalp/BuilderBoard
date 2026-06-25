//! Generalized ExecutionRequest.
//!
//! Replaces the chat-centric ProviderRequest. Supports multiple execution kinds
//! so that future engines (Grok, Claude, local, image, embedding, skills, voice)
//! can be routed without changing the core boundary.

use crate::execution::tool_transport::NativeToolDefinition;
use crate::models::{Conversation, Model};

/// Chat-style request (current primary use case for OpenAI compatibility).
#[derive(Clone, Debug)]
pub struct ChatRequest {
    pub conversation: Conversation,
    pub reasoning_level: Option<String>,
    pub native_tools: Vec<NativeToolDefinition>,
    pub trace_round: Option<u32>,
    // Future: temperature, etc. without changing the enum variant.
}

/// Simple completion (non-chat).
#[derive(Clone, Debug)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: Model,
    pub max_tokens: Option<u32>,
}

/// Embedding request.
#[derive(Clone, Debug)]
pub struct EmbedRequest {
    pub inputs: Vec<String>,
    pub model: Option<String>,
}

/// Image generation / editing request (placeholder for future modalities).
#[derive(Clone, Debug)]
pub struct ImageRequest {
    pub prompt: String,
    pub image: Option<Vec<u8>>, // for edit
    pub size: Option<String>,
    pub model: Option<String>,
}

/// Tool / function execution request (direct, not via chat).
#[derive(Clone, Debug)]
pub struct ToolRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Structured output request.
#[derive(Clone, Debug)]
pub struct StructuredRequest {
    pub schema: serde_json::Value, // JSON Schema
    pub prompt: String,
}

/// Raw / engine-specific request for future-proofing.
#[derive(Clone, Debug)]
pub struct RawRequest {
    pub payload: serde_json::Value,
}

/// The top-level polymorphic request passed to ExecutionEngine::execute.
///
/// Engines inspect the variant and return "unsupported" if they cannot handle it.
#[derive(Clone, Debug)]
pub enum ExecutionRequest {
    Chat(ChatRequest),
    Completion(CompletionRequest),
    Embed(EmbedRequest),
    Image(ImageRequest),
    Tool(ToolRequest),
    Structured(StructuredRequest),
    Raw(RawRequest),
}

impl ExecutionRequest {
    /// Convenience constructor for current chat path.
    pub fn chat(conversation: Conversation, reasoning_level: Option<String>) -> Self {
        Self::Chat(ChatRequest {
            conversation,
            reasoning_level,
            native_tools: vec![],
            trace_round: None,
        })
    }

    pub fn chat_with_native_tools(
        conversation: Conversation,
        reasoning_level: Option<String>,
        native_tools: Vec<NativeToolDefinition>,
    ) -> Self {
        Self::Chat(ChatRequest {
            conversation,
            reasoning_level,
            native_tools,
            trace_round: None,
        })
    }

    pub fn chat_with_native_tools_for_round(
        conversation: Conversation,
        reasoning_level: Option<String>,
        native_tools: Vec<NativeToolDefinition>,
        trace_round: u32,
    ) -> Self {
        Self::Chat(ChatRequest {
            conversation,
            reasoning_level,
            native_tools,
            trace_round: Some(trace_round),
        })
    }

    /// Returns a human-readable kind for logging/routing.
    pub fn kind(&self) -> &'static str {
        match self {
            ExecutionRequest::Chat(_) => "chat",
            ExecutionRequest::Completion(_) => "completion",
            ExecutionRequest::Embed(_) => "embed",
            ExecutionRequest::Image(_) => "image",
            ExecutionRequest::Tool(_) => "tool",
            ExecutionRequest::Structured(_) => "structured",
            ExecutionRequest::Raw(_) => "raw",
        }
    }
}
