//! Execution Engine abstraction (Phase 8.9A + 8.9A.1 Generalization).
//!
//! Execution-centric boundary (not chat or HTTP centric).
//! Supports future engines: OpenAI, Grok Build, Claude, local (Ollama, MLX, LM Studio),
//! embeddings, images, skills, voice, subagents, etc.

pub mod capabilities;
pub mod capability_resolver;
pub mod cli;
pub mod context;
pub mod engine;
pub mod event;
pub mod grok_build;
pub mod manager;
pub mod request;
pub mod tool_engine;
pub mod tool_transport;
pub mod tools;

pub use engine::{global_engine_registry, EngineRegistry, ExecutionEngine, OpenAIExecutionEngine};
pub use grok_build::GrokBuildExecutionEngine;
pub use tool_engine::ToolExecutionEngine;

// Re-export primary new types
pub use capabilities::EngineCapabilities;
pub use capability_resolver::{
    audit_capabilities, build_comprehensive_tool_advertisement, build_tool_advertisement,
    parse_tool_calls, resolve_allowed_tools, summarize_capabilities, tool_input_schema,
    tool_permission_allowed, tool_usage_examples, AuditReport,
};
pub use context::ExecutionContext;
pub use event::ExecutionEvent;
pub use manager::{ExecutionClass, ExecutionManager, ExecutionProfile, ExecutionResolution};
pub use request::ExecutionRequest;
pub use tool_transport::{native_tool_definitions, NativeToolCall, NativeToolDefinition};

// Re-export Tool Runtime
pub use tools::registry::global_tool_registry;
pub use tools::traits::Tool;
