//! Execution Engine abstraction (Phase 8.9A + 8.9A.1 Generalization).
//!
//! Execution-centric boundary (not chat or HTTP centric).
//! Supports future engines: OpenAI, Grok Build, Claude, local (Ollama, MLX, LM Studio),
//! embeddings, images, skills, voice, subagents, etc.

pub mod capabilities;
pub mod cli;
pub mod context;
pub mod engine;
pub mod event;
pub mod grok_build;
pub mod manager;
pub mod request;

pub use engine::{
    EngineRegistry, ExecutionEngine, OpenAIExecutionEngine, global_engine_registry,
};
pub use grok_build::GrokBuildExecutionEngine;

// Re-export primary new types
pub use capabilities::EngineCapabilities;
pub use context::ExecutionContext;
pub use event::ExecutionEvent;
pub use manager::{ExecutionClass, ExecutionManager, ExecutionProfile, ExecutionResolution};
pub use request::ExecutionRequest;
