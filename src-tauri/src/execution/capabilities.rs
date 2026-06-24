//! Expanded EngineCapabilities.
//!
//! Describes what an engine can do, for routing, UI, and planning.
//! This replaces the minimal previous model and is inspired by
//! ProviderDto + SKILL_SPEC execution profiles + future needs (skills, subagents, etc.).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Locality {
    #[default]
    Remote,
    Local,
    Hybrid,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    #[default]
    Http,
    Sse,
    ProcessCli,
    Stdio,
    UnixSocket,
    Embedded,
    Ipc,
    Other(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SupportedFeatures {
    pub chat: bool,
    pub streaming: bool,
    pub reasoning: bool,
    pub tool_use: bool,
    pub images: bool,
    pub embeddings: bool,
    pub structured_output: bool,
    pub multimodal: bool,
    pub filesystem: bool,
    pub shell: bool,
    pub subagents: bool,
    pub worktrees: bool,
    pub cancellation: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContextLimits {
    pub max_context_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub gpu: bool,
    pub min_ram_gb: Option<u32>,
    pub model_size_hint: Option<String>,
}

/// Rich capabilities advertised by an execution engine.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EngineCapabilities {
    pub engine_id: String,

    /// Where the engine runs.
    pub locality: Locality,

    /// How the engine communicates (implementation detail, but useful for routing).
    pub supported_transports: Vec<Transport>,

    /// What the engine can do.
    pub features: SupportedFeatures,

    /// Limits.
    pub context_limits: ContextLimits,

    /// Resource needs.
    pub resources: ResourceRequirements,

    /// Free-form tags for future routing (e.g. ["xai", "local", "fast"]).
    pub tags: Vec<String>,

    /// Human readable notes.
    pub description: Option<String>,
}

impl EngineCapabilities {
    pub fn for_openai() -> Self {
        Self {
            engine_id: "openai".to_string(),
            locality: Locality::Remote,
            supported_transports: vec![Transport::Http, Transport::Sse],
            features: SupportedFeatures {
                chat: true,
                streaming: true,
                reasoning: true,
                tool_use: false, // current phase
                images: false,
                embeddings: false,
                structured_output: false,
                multimodal: false,
                filesystem: false,
                shell: false,
                subagents: false,
                worktrees: false,
                cancellation: true,
            },
            context_limits: ContextLimits {
                max_context_tokens: Some(128_000),
                max_output_tokens: Some(4096),
            },
            resources: ResourceRequirements::default(),
            tags: vec!["http".to_string(), "remote".to_string()],
            description: Some("OpenAI Chat Completions (including ChatGPT OAuth backend)".to_string()),
        }
    }
}
