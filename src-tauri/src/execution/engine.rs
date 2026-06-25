//! Core ExecutionEngine trait and supporting types (generalized).
//!
//! This is the heart of Phase 8.9A.1. The trait and models are now execution-centric,
//! not chat-HTTP-centric. OpenAI is adapted internally so its observable runtime
//! behavior remains 100% unchanged.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use crate::providers::{ProviderError, ProviderResult}; // kept for OpenAI internal compatibility

// Re-export the primary new types from their modules
pub use super::capabilities::EngineCapabilities;
pub use super::context::ExecutionContext;
pub use super::event::ExecutionEvent;
pub use super::request::ExecutionRequest;

// Result type for generalized execution.
pub type ExecutionResult = Result<(), ExecutionError>;

#[derive(Clone, Debug)]
pub enum ExecutionError {
    UnsupportedRequest { kind: String },
    NotReady { message: String },
    Internal { message: String },
    Cancelled,
}

impl From<ProviderError> for ExecutionError {
    fn from(e: ProviderError) -> Self {
        ExecutionError::Internal {
            message: format!("{:?}", e),
        }
    }
}

/// The generalized ExecutionEngine trait.
///
/// Engines implement this. The runtime (and future Skills/Agents) call execute().
/// Transport, auth, modality, and chat vs non-chat are all engine implementation details.
pub trait ExecutionEngine: Send + Sync {
    fn engine_id(&self) -> &'static str;

    fn capabilities(&self) -> EngineCapabilities;

    fn display_name(&self) -> String {
        self.engine_id().to_string()
    }

    fn list_models(&self) -> Vec<String> {
        vec![]
    }

    fn supported_effort_levels(&self) -> Vec<String> {
        vec!["medium".to_string()]
    }

    fn health(&self) -> String {
        "available".to_string()
    }

    fn supports_native_tools(&self) -> bool {
        false
    }

    /// Primary generic entry point.
    ///
    /// The engine receives a rich context (project/fs/optional creds/cancellation)
    /// and a polymorphic request. It emits normalized events.
    ///
    /// Engines should be cooperative with cancellation.
    fn execute(
        &self,
        context: ExecutionContext,
        request: ExecutionRequest,
        on_event: Box<dyn Fn(ExecutionEvent) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send>>;

    /// Optional: engines can declare which request kinds they support.
    fn supports(&self, request: &ExecutionRequest) -> bool {
        // Default conservative implementation; engines should override.
        match request {
            ExecutionRequest::Chat(_) => self.capabilities().features.chat,
            _ => false,
        }
    }
}

// ===== OpenAI implementation (adapted for new boundary, identical behavior) =====

use crate::chat::ProviderResolutionService;
use crate::providers::{ProviderRequest as OldProviderRequest, StreamChunk as OldStreamChunk};

/// OpenAIExecutionEngine adapted to the generalized boundary.
///
/// Internally it still uses the proven OpenAIProvider streaming logic.
/// Externally it speaks the new ExecutionRequest / ExecutionEvent / ExecutionContext model.
/// This keeps all runtime behavior (deltas, persistence, metrics, traces) identical.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenAIExecutionEngine;

impl OpenAIExecutionEngine {
    pub fn new() -> Self {
        Self
    }
}

impl ExecutionEngine for OpenAIExecutionEngine {
    fn engine_id(&self) -> &'static str {
        "openai"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities::for_openai()
    }

    fn display_name(&self) -> String {
        "OpenAI".to_string()
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "GPT-5.5".to_string(),
            "GPT-5.5 Thinking".to_string(),
            "gpt-4o-mini".to_string(),
            "GPT-5.4 mini".to_string(),
            "GPT-5.3 Codex Spark".to_string(),
        ]
    }

    fn supported_effort_levels(&self) -> Vec<String> {
        vec![
            "low".to_string(),
            "medium".to_string(),
            "high".to_string(),
            "max".to_string(),
        ]
    }

    fn health(&self) -> String {
        "available".to_string()
    }

    fn supports_native_tools(&self) -> bool {
        true
    }

    fn execute(
        &self,
        context: ExecutionContext,
        request: ExecutionRequest,
        on_event: Box<dyn Fn(ExecutionEvent) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send>> {
        // Only support Chat for now (matching current BuilderBoard usage).
        let chat_req = match request {
            ExecutionRequest::Chat(c) => c,
            other => {
                let kind = other.kind().to_string();
                return Box::pin(async move { Err(ExecutionError::UnsupportedRequest { kind }) });
            }
        };

        // Convert to old internal types (preserves exact OpenAI logic and behavior)
        let old_request = OldProviderRequest::new(chat_req.conversation)
            .with_reasoning_level(chat_req.reasoning_level)
            .with_native_tools(chat_req.native_tools)
            .with_trace_round(chat_req.trace_round);

        // Build a minimal legacy context for the credential resolution path we already have.
        // We only use the credential parts; the rest of the new context (fs, project) is
        // handled outside the engine today (enrichment).
        let legacy_ctx = crate::chat::PaneExecutionContext {
            provider: crate::storage::models::ProviderDto {
                id: "openai".to_string(),
                provider_type: "openai".to_string(),
                display_name: "OpenAI".to_string(),
                enabled: true,
                auth_mode: "api_key".to_string(),
                supports_chat: true,
                supports_streaming: true,
                supports_tool_use: false,
                supports_vision: false,
                context_window: Some(128000),
                locality: "remote".to_string(),
            },
            credential: context.credential.clone().unwrap_or_else(|| {
                // Fallback — real path always supplies one via previous resolution
                crate::auth::CredentialHandle {
                    provider_id: "openai".to_string(),
                    account_id: "openai".to_string(),
                    auth_type: "api_key".to_string(),
                    credential_ref: "".to_string(),
                    token_expires_at: None,
                }
            }),
            oauth_external_account_id: None,
        };

        let creds = context.credential_service.clone();

        Box::pin(async move {
            // Use the existing credential-bound provider (exact same path as before)
            let provider = match creds {
                Some(cs) => {
                    match ProviderResolutionService::resolve_openai_provider(legacy_ctx, &cs) {
                        Ok(p) => p,
                        Err(e) => {
                            return Err(ExecutionError::Internal {
                                message: format!("{:?}", e),
                            })
                        }
                    }
                }
                None => {
                    // For pure local future engines this would be fine, but OpenAI needs it.
                    return Err(ExecutionError::Internal {
                        message: "OpenAI engine requires credential service".to_string(),
                    });
                }
            };

            // Emit start
            on_event(ExecutionEvent::RunStarted {
                execution_id: context.execution_id.clone(),
                engine_id: "openai".to_string(),
                request_kind: "chat".to_string(),
            });

            // Bridge old streaming chunks → new events (exact same deltas)
            let bridge_result = provider
                .stream_chunks_async(old_request, |old_chunk: ProviderResult<OldStreamChunk>| {
                    match old_chunk {
                        Ok(chunk) => {
                            if !chunk.content_delta.is_empty() {
                                on_event(ExecutionEvent::TextDelta {
                                    content: chunk.content_delta,
                                });
                            }
                            for tool_call in chunk.tool_calls {
                                on_event(ExecutionEvent::ToolCallStarted {
                                    call_id: tool_call.call_id,
                                    name: tool_call.tool_name,
                                    arguments: Some(tool_call.arguments.to_string()),
                                });
                            }
                            if chunk.is_complete {
                                on_event(ExecutionEvent::RunCompleted {
                                    execution_id: context.execution_id.clone(),
                                    success: true,
                                    summary: None,
                                });
                            }
                            Ok(())
                        }
                        Err(e) => {
                            on_event(ExecutionEvent::Error {
                                code: "provider_error".to_string(),
                                message: format!("{:?}", e),
                            });
                            Err(e)
                        }
                    }
                })
                .await;

            match bridge_result {
                Ok(()) => Ok(()),
                Err(e) => Err(ExecutionError::from(e)),
            }
        })
    }
}

// ===== Registry (largely unchanged, now with richer capabilities) =====

use std::collections::HashMap;

static REGISTRY: OnceLock<EngineRegistry> = OnceLock::new();

pub struct EngineRegistry {
    engines: HashMap<String, Arc<dyn ExecutionEngine>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    pub fn register(&mut self, engine: Arc<dyn ExecutionEngine>) {
        self.engines.insert(engine.engine_id().to_string(), engine);
    }

    pub fn get(&self, engine_id: &str) -> Option<Arc<dyn ExecutionEngine>> {
        self.engines.get(engine_id).cloned()
    }

    pub fn list_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }
}

pub fn global_engine_registry() -> &'static EngineRegistry {
    REGISTRY.get_or_init(|| {
        let mut reg = EngineRegistry::new();
        register_default_engines(&mut reg);
        reg
    })
}

pub fn register_default_engines(registry: &mut EngineRegistry) {
    registry.register(Arc::new(OpenAIExecutionEngine::new()));
    // Grok Build via CLI (registered as "grok" so it can be selected by model or provider)
    registry.register(Arc::new(
        crate::execution::grok_build::GrokBuildExecutionEngine::new(),
    ));
    // Tool Runtime engine — routes ExecutionRequest::Tool → ToolRegistry → Tool
    registry.register(Arc::new(
        crate::execution::tool_engine::ToolExecutionEngine::new(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::capabilities::Locality;

    #[test]
    fn openai_engine_advertises_rich_capabilities() {
        let eng = OpenAIExecutionEngine::new();
        let caps = eng.capabilities();
        assert_eq!(caps.locality, Locality::Remote);
        assert!(caps.features.chat);
        assert!(caps.features.streaming);
        assert!(caps.features.reasoning);
    }

    #[test]
    fn registry_still_works() {
        let reg = global_engine_registry();
        assert!(reg.get("openai").is_some());
    }
}
