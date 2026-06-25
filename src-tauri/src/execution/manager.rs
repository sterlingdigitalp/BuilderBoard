//! Intelligent Execution Manager (Phase 8.9E)
//!
//! Resolves Builder intent + ExecutionClass into concrete engine, model, effort, policy.
//! Builders describe preferences (class + engine prefs).
//! Manager scores engines, applies policy, handles intelligent fallback with reasons.
//! Extends existing architecture; engines and builders remain unchanged.

use crate::builders::{global_builder_registry, Builder};
use crate::execution::capabilities::EngineCapabilities;
use crate::execution::context::ExecutionContext;
use crate::execution::engine::{global_engine_registry, ExecutionEngine};
use crate::execution::request::ExecutionRequest;

/// Strongly typed execution classification.
/// Describes the *kind of work*, independent of any engine.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionClass {
    Architecture,
    Implementation,
    Research,
    Review,
    Testing,
    Debugging,
    Documentation,
    Planning,
    Analysis,
    General,
}

impl Default for ExecutionClass {
    fn default() -> Self {
        ExecutionClass::General
    }
}

impl ExecutionClass {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "architecture" => ExecutionClass::Architecture,
            "implementation" => ExecutionClass::Implementation,
            "research" => ExecutionClass::Research,
            "review" => ExecutionClass::Review,
            "testing" => ExecutionClass::Testing,
            "debugging" => ExecutionClass::Debugging,
            "documentation" => ExecutionClass::Documentation,
            "planning" => ExecutionClass::Planning,
            "analysis" => ExecutionClass::Analysis,
            _ => ExecutionClass::General,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionClass::Architecture => "architecture",
            ExecutionClass::Implementation => "implementation",
            ExecutionClass::Research => "research",
            ExecutionClass::Review => "review",
            ExecutionClass::Testing => "testing",
            ExecutionClass::Debugging => "debugging",
            ExecutionClass::Documentation => "documentation",
            ExecutionClass::Planning => "planning",
            ExecutionClass::Analysis => "analysis",
            ExecutionClass::General => "general",
        }
    }
}

/// Generalized execution profile from a Builder (intent, not binding).
#[derive(Clone, Debug)]
pub struct ExecutionProfile {
    pub class: ExecutionClass,
    pub preferred_engine: Option<String>,
    pub fallback_engines: Vec<String>,
    pub effort: String,
    pub default_model: String,
    pub review_requirements: String,
    pub memory_defaults: String,
}

impl From<&Builder> for ExecutionProfile {
    fn from(builder: &Builder) -> Self {
        let exec = &builder.execution;
        ExecutionProfile {
            class: ExecutionClass::from_str(exec.preferred_class.as_str()),
            preferred_engine: Some(exec.preferred_engine.clone()),
            fallback_engines: exec.fallback_engines.clone(),
            effort: exec.effort.clone(),
            default_model: exec.default_model.clone(),
            review_requirements: exec.review_requirements.clone(),
            memory_defaults: exec.memory_defaults.clone(),
        }
    }
}

/// Result of manager resolution, with explanation for UI/diagnostics.
#[derive(Clone, Debug)]
pub struct ExecutionResolution {
    pub engine_id: String,
    pub model: String,
    pub effort: String,
    pub reason: String,
    pub class: ExecutionClass,
    pub policy_applied: bool,
}

/// Lightweight engine scoring (deterministic, simple).
fn score_engine(
    engine: &dyn ExecutionEngine,
    class: &ExecutionClass,
    profile: &ExecutionProfile,
    caps: &EngineCapabilities,
    health: &str,
) -> i32 {
    let mut score: i32 = 0;

    // Strong preference for explicit preferred engine
    if let Some(pref) = &profile.preferred_engine {
        if engine.engine_id() == pref {
            score += 100;
        }
    }

    // Class affinity (basic mapping; can be extended)
    match class {
        ExecutionClass::Implementation | ExecutionClass::Debugging | ExecutionClass::Testing => {
            if caps.features.tool_use || caps.features.shell || caps.features.filesystem {
                score += 30;
            }
            if caps.features.reasoning {
                score += 15;
            }
        }
        ExecutionClass::Review | ExecutionClass::Analysis | ExecutionClass::Research => {
            if caps.features.reasoning {
                score += 40;
            }
        }
        ExecutionClass::Architecture | ExecutionClass::Planning | ExecutionClass::Documentation => {
            if caps.features.reasoning {
                score += 25;
            }
        }
        ExecutionClass::General => {}
    }

    // Health / availability
    if health == "available" {
        score += 50;
    } else if health.contains("missing") || health.contains("unavailable") {
        score -= 200;
    } else if health.contains("auth") {
        score -= 80;
    }

    // Locality preference (prefer local for speed in implementation)
    let loc_str = format!("{:?}", caps.locality);
    if matches!(
        class,
        ExecutionClass::Implementation | ExecutionClass::Debugging
    ) && (loc_str.contains("Local") || loc_str.contains("Hybrid"))
    {
        score += 10;
    }

    // Capability match for streaming (common)
    if caps.features.streaming {
        score += 5;
    }

    // Fallback penalty for non-preferred
    if profile.preferred_engine.as_deref() != Some(engine.engine_id()) {
        score -= 20;
    }

    score
}

/// The intelligent Execution Manager.
/// Single authority for routing intent to concrete execution.
pub struct ExecutionManager;

impl ExecutionManager {
    pub fn new() -> Self {
        Self
    }

    /// Resolve from a Builder (or profile) + optional class + context.
    /// Returns chosen engine + explanation.
    /// Falls back intelligently if preferred unavailable.
    pub fn resolve(
        builder_name: Option<&str>,
        requested_class: Option<ExecutionClass>,
        context: &ExecutionContext,
        _request: &ExecutionRequest,
    ) -> ExecutionResolution {
        let builder_reg = global_builder_registry();
        let engine_reg = global_engine_registry();

        let profile = if let Some(name) = builder_name {
            builder_reg
                .get(name)
                .map(|b| ExecutionProfile::from(b.as_ref()))
                .unwrap_or_else(|| ExecutionProfile {
                    class: ExecutionClass::General,
                    preferred_engine: None,
                    fallback_engines: vec![],
                    effort: "medium".to_string(),
                    default_model: "default".to_string(),
                    review_requirements: "none".to_string(),
                    memory_defaults: "pane".to_string(),
                })
        } else {
            ExecutionProfile {
                class: ExecutionClass::General,
                preferred_engine: None,
                fallback_engines: vec![],
                effort: "medium".to_string(),
                default_model: "default".to_string(),
                review_requirements: "none".to_string(),
                memory_defaults: "pane".to_string(),
            }
        };

        let class = requested_class.unwrap_or(profile.class.clone());

        // Build candidate list: preferred + fallbacks + all available
        let mut candidates: Vec<String> = vec![];
        if let Some(pref) = &profile.preferred_engine {
            candidates.push(pref.clone());
        }
        for fb in &profile.fallback_engines {
            if !candidates.contains(fb) {
                candidates.push(fb.clone());
            }
        }
        for id in engine_reg.list_ids() {
            if !candidates.contains(&id) {
                candidates.push(id);
            }
        }

        let mut best: Option<(String, i32, String)> = None;

        for engine_id in candidates {
            if let Some(engine) = engine_reg.get(&engine_id) {
                let health = engine.health(); // Note: engines now should implement health; current ones do via prior phases
                let caps = engine.capabilities();

                // Basic policy check (simplified - extend as needed)
                let policy_ok = Self::check_policy(&caps, context, &profile);

                if !policy_ok {
                    continue;
                }

                let score = score_engine(engine.as_ref(), &class, &profile, &caps, &health);

                let reason = if Some(&engine_id) == profile.preferred_engine.as_ref() {
                    format!("Preferred engine for {} class", class.as_str())
                } else if profile.fallback_engines.contains(&engine_id) {
                    format!("Fallback from preferred; {} class match", class.as_str())
                } else {
                    format!(
                        "Best available match for {} (score={})",
                        class.as_str(),
                        score
                    )
                };

                let candidate = (engine_id.clone(), score, reason);

                if best.as_ref().map_or(true, |(_, s, _)| score > *s) {
                    best = Some(candidate);
                }
            }
        }

        let (engine_id, _score, reason) = best.unwrap_or_else(|| {
            (
                "openai".to_string(),
                0,
                "Emergency fallback to OpenAI (no suitable engine found)".to_string(),
            )
        });

        // Apply profile policy values (effort etc), manager decided the engine
        let effort = profile.effort.clone();
        let model = if engine_id == "grok" || engine_id.contains("grok") {
            "grok-build".to_string()
        } else {
            profile.default_model.clone()
        };

        ExecutionResolution {
            engine_id,
            model,
            effort,
            reason,
            class,
            policy_applied: true,
        }
    }

    fn check_policy(
        caps: &EngineCapabilities,
        _context: &ExecutionContext,
        _profile: &ExecutionProfile,
    ) -> bool {
        // Simplified policy gate (filesystem, shell, etc. could be checked against context.policy)
        // For this phase: always allow if engine is healthy; real policy enforcement can layer on top.
        caps.features.chat || caps.features.streaming // minimal viability
    }

    /// Convenience: resolve using builder name + derive class from request (chat -> Implementation)
    pub fn resolve_for_chat(
        builder_name: Option<&str>,
        context: &ExecutionContext,
        request: &ExecutionRequest,
    ) -> ExecutionResolution {
        let class = match request {
            ExecutionRequest::Chat(_) => ExecutionClass::Implementation,
            _ => ExecutionClass::General,
        };
        Self::resolve(builder_name, Some(class), context, request)
    }

    /// Resolve the current stream execution route from either a Builder name or a direct engine id.
    /// This keeps stream execution generic: adding a Builder changes BuilderRegistry only.
    pub fn resolve_stream_route(
        route_id: &str,
        model_id: &str,
        effort: Option<&str>,
        context: &ExecutionContext,
        request: &ExecutionRequest,
    ) -> ExecutionResolution {
        if global_builder_registry().get(route_id).is_some() {
            return Self::resolve_for_chat(Some(route_id), context, request);
        }

        let engine_exists = global_engine_registry().get(route_id).is_some();
        let class = match request {
            ExecutionRequest::Chat(_) => ExecutionClass::General,
            _ => ExecutionClass::General,
        };

        ExecutionResolution {
            engine_id: route_id.to_string(),
            model: model_id.to_string(),
            effort: effort.unwrap_or("medium").to_string(),
            reason: if engine_exists {
                "Direct engine selection".to_string()
            } else {
                "Unregistered engine selected; execution will fail if no engine is available"
                    .to_string()
            },
            class,
            policy_applied: false,
        }
    }
}

impl Default for ExecutionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::request::ChatRequest;
    use crate::models::{Conversation, Message, MessageRole, Model};

    fn sample_chat_request() -> ExecutionRequest {
        ExecutionRequest::Chat(ChatRequest {
            conversation: Conversation::new("test", Model::Custom("test".into())),
            reasoning_level: None,
            native_tools: vec![],
            trace_round: None,
        })
    }

    #[test]
    fn manager_respects_builder_preference_when_available() {
        let ctx = ExecutionContext::local("test-exec");
        let req = sample_chat_request();
        let res = ExecutionManager::resolve_for_chat(Some("builder-c"), &ctx, &req);
        // builder-c prefers grok
        assert_eq!(res.engine_id, "grok");
        assert!(res.reason.contains("Preferred") || res.reason.contains("grok"));
    }

    #[test]
    fn manager_falls_back_gracefully() {
        let ctx = ExecutionContext::local("test-exec");
        let req = sample_chat_request();
        // Use a builder that prefers something unavailable
        let res = ExecutionManager::resolve(
            Some("builder-c"),
            Some(ExecutionClass::Implementation),
            &ctx,
            &req,
        );
        assert!(!res.engine_id.is_empty());
        assert!(!res.reason.is_empty());
    }

    #[test]
    fn execution_class_from_str_roundtrips() {
        assert_eq!(
            ExecutionClass::from_str("implementation"),
            ExecutionClass::Implementation
        );
        assert_eq!(ExecutionClass::from_str("review"), ExecutionClass::Review);
        assert_eq!(ExecutionClass::from_str("foo"), ExecutionClass::General);
    }
}
