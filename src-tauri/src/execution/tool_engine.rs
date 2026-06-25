use std::future::Future;
use std::pin::Pin;

use crate::execution::capabilities::{
    ContextLimits, EngineCapabilities, Locality, ResourceRequirements, SupportedFeatures, Transport,
};
use crate::execution::context::ExecutionContext;
use crate::execution::engine::{ExecutionEngine, ExecutionError, ExecutionResult};
use crate::execution::event::ExecutionEvent;
use crate::execution::request::ExecutionRequest;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::registry::global_tool_registry;

pub struct ToolExecutionEngine;

impl ToolExecutionEngine {
    pub fn new() -> Self {
        Self
    }
}

impl ExecutionEngine for ToolExecutionEngine {
    fn engine_id(&self) -> &'static str {
        "tool"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            engine_id: "tool".to_string(),
            locality: Locality::Local,
            supported_transports: vec![Transport::Embedded],
            features: SupportedFeatures {
                chat: false,
                streaming: true,
                reasoning: false,
                tool_use: true,
                images: false,
                embeddings: false,
                structured_output: false,
                multimodal: false,
                filesystem: true,
                shell: true,
                subagents: false,
                worktrees: false,
                cancellation: true,
            },
            context_limits: ContextLimits::default(),
            resources: ResourceRequirements::default(),
            tags: vec!["embedded".to_string(), "local".to_string()],
            description: Some(
                "Tool Runtime Engine — routes ExecutionRequest::Tool through ToolRegistry"
                    .to_string(),
            ),
        }
    }

    fn display_name(&self) -> String {
        "Tool Runtime Engine".to_string()
    }

    fn health(&self) -> String {
        let count = global_tool_registry()
            .read()
            .map(|reg| reg.len())
            .unwrap_or(0);
        if count > 0 {
            format!("available ({} tools registered)", count)
        } else {
            "no tools registered".to_string()
        }
    }

    fn supports(&self, request: &ExecutionRequest) -> bool {
        matches!(request, ExecutionRequest::Tool(_))
    }

    fn execute(
        &self,
        context: ExecutionContext,
        request: ExecutionRequest,
        on_event: Box<dyn Fn(ExecutionEvent) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send>> {
        let tool_req = match request {
            ExecutionRequest::Tool(t) => t,
            other => {
                let kind = other.kind().to_string();
                return Box::pin(async move { Err(ExecutionError::UnsupportedRequest { kind }) });
            }
        };

        Box::pin(async move {
            on_event(ExecutionEvent::RunStarted {
                execution_id: context.execution_id.clone(),
                engine_id: "tool".to_string(),
                request_kind: "tool".to_string(),
            });

            let registry = global_tool_registry();
            let reg = registry.read().map_err(|e| ExecutionError::Internal {
                message: format!("ToolRegistry lock error: {}", e),
            })?;

            let tool =
                reg.get(&tool_req.tool_name)
                    .ok_or_else(|| ExecutionError::UnsupportedRequest {
                        kind: format!("unknown tool '{}'", tool_req.tool_name),
                    })?;

            let tool_ctx = ToolContext {
                execution_id: context.execution_id.clone(),
                pane_id: context.pane_id.clone(),
                project_root: context.project.as_ref().map(|p| p.approved_root.clone()),
                filesystem_scope: context.filesystem_scope.clone(),
                cwd: context
                    .cwd
                    .clone()
                    .or_else(|| context.project.as_ref().map(|p| p.approved_root.clone())),
                environment: context.environment.clone(),
                cancellation: context.cancellation.clone(),
                timeout_ms: context.policy.timeout_ms,
                allow_shell: context.policy.allow_shell,
                allow_network: context.policy.allow_network,
                allow_read: context.policy.allow_read,
                allow_write: context.policy.allow_write,
                allow_delete: context.policy.allow_delete,
                allow_git: context.policy.allow_git,
                allow_packages: context.policy.allow_packages,
                allow_processes: context.policy.allow_processes,
            };

            tool.validate(&tool_req.arguments)
                .map_err(|e| ExecutionError::Internal {
                    message: format!("Tool validation failed: {}", e),
                })?;

            let result = tool
                .execute(tool_ctx, tool_req.arguments.clone(), &|event| {
                    on_event(event);
                })
                .map_err(|e| ExecutionError::Internal {
                    message: format!("Tool execution failed: {}", e),
                })?;

            for artifact in &result.artifacts {
                on_event(ExecutionEvent::ArtifactCreated {
                    artifact_type: artifact.artifact_type.clone(),
                    summary: artifact.summary.clone(),
                    content_ref: artifact.path.clone(),
                });
            }

            for item in &result.review_items {
                on_event(ExecutionEvent::ReviewItemCreated {
                    tool_id: tool_req.tool_name.clone(),
                    execution_id: context.execution_id.clone(),
                    action: item.action.clone(),
                    summary: item.summary.clone(),
                    details: item.details.clone(),
                });
            }

            on_event(ExecutionEvent::RunCompleted {
                execution_id: context.execution_id.clone(),
                success: result.success,
                summary: Some(result.output.summary.clone()),
            });

            Ok(())
        })
    }
}
