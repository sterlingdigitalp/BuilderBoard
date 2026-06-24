//! Generalized ExecutionEvent model.
//!
//! Replaces the chat-specific StreamChunk. Designed for all future engines
//! and modalities. Events are emitted by the engine during execute().

use serde::{Deserialize, Serialize};

/// Normalized event emitted during any execution (chat, tool, image, skill, etc.).
///
/// The model is intentionally broad. UIs and orchestrators filter the events
/// they care about.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// Execution run has begun.
    RunStarted {
        execution_id: String,
        engine_id: String,
        request_kind: String,
    },

    /// Generic status update.
    Status {
        message: String,
    },

    /// Incremental reasoning / chain-of-thought (visible or hidden depending on UI).
    ReasoningDelta {
        content: String,
    },

    /// Incremental text output (the primary "token" stream for chat/completion).
    TextDelta {
        content: String,
    },

    /// Progress indicator (for long operations, image gen, indexing, etc.).
    Progress {
        current: u64,
        total: Option<u64>,
        message: Option<String>,
    },

    /// A tool call is starting.
    ToolCallStarted {
        call_id: String,
        name: String,
        arguments: Option<String>,
    },

    /// A tool call has finished.
    ToolCallFinished {
        call_id: String,
        result: Option<String>,
        error: Option<String>,
    },

    /// An artifact (structured output, file, image, etc.) was produced.
    ArtifactCreated {
        artifact_type: String,
        summary: String,
        // content is intentionally not inlined here for large artifacts
        content_ref: Option<String>,
    },

    /// Non-fatal warning.
    Warning {
        message: String,
    },

    /// Fatal or terminal error.
    Error {
        code: String,
        message: String,
    },

    /// The run was cancelled.
    Cancelled {
        reason: Option<String>,
    },

    /// Execution completed successfully (no more events for this run).
    RunCompleted {
        execution_id: String,
        success: bool,
        summary: Option<String>,
    },

    /// Engine-specific metadata / trace info.
    EngineMetadata {
        key: String,
        value: String,
    },
}

impl ExecutionEvent {
    pub fn text_delta(content: impl Into<String>) -> Self {
        ExecutionEvent::TextDelta {
            content: content.into(),
        }
    }

    pub fn run_completed(execution_id: impl Into<String>, success: bool) -> Self {
        ExecutionEvent::RunCompleted {
            execution_id: execution_id.into(),
            success,
            summary: None,
        }
    }
}
