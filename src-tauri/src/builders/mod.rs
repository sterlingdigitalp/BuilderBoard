//! Builder System (Phase 8.9D)
//!
//! Builders define execution policies, preferred engines, models, efforts, etc.
//! Loaded dynamically. Panes select Builders which auto-configure execution.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuilderExecutionPolicy {
    #[serde(default = "default_class")]
    pub preferred_class: String, // e.g. "implementation", "review"
    pub preferred_engine: String,
    #[serde(default)]
    pub fallback_engines: Vec<String>,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default)]
    pub review_requirements: String, // e.g. "human", "auto", "none"
    #[serde(default)]
    pub memory_defaults: String, // e.g. "project:shared"
}

fn default_class() -> String {
    "implementation".to_string()
}

fn default_effort() -> String {
    "medium".to_string()
}
fn default_model() -> String {
    "default".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Builder {
    pub name: String,
    pub display_name: String,
    pub execution: BuilderExecutionPolicy,
    // Future: more fields from BUILDER.yaml
}

impl Builder {
    pub fn preferred_engine(&self) -> &str {
        &self.execution.preferred_engine
    }

    pub fn effective_engine(&self) -> &str {
        // For now return preferred; fallbacks handled at selection time
        self.preferred_engine()
    }

    pub fn effort(&self) -> &str {
        &self.execution.effort
    }

    pub fn default_model(&self) -> &str {
        &self.execution.default_model
    }
}

pub struct BuilderRegistry {
    builders: HashMap<String, Arc<Builder>>,
}

impl BuilderRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    pub fn register(&mut self, builder: Builder) {
        self.builders
            .insert(builder.name.clone(), Arc::new(builder));
    }

    pub fn get(&self, name: &str) -> Option<Arc<Builder>> {
        self.builders.get(name).cloned()
    }

    pub fn list(&self) -> Vec<Arc<Builder>> {
        let mut list: Vec<_> = self.builders.values().cloned().collect();
        list.sort_by_key(|b| b.name.clone());
        list
    }

    pub fn list_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.builders.keys().cloned().collect();
        names.sort();
        names
    }
}

static REGISTRY: OnceLock<BuilderRegistry> = OnceLock::new();

pub fn global_builder_registry() -> &'static BuilderRegistry {
    REGISTRY.get_or_init(|| {
        let mut reg = BuilderRegistry::new();
        register_default_builders(&mut reg);
        reg
    })
}

fn register_default_builders(registry: &mut BuilderRegistry) {
    // Builder A: Full review, OpenAI primary (from history)
    registry.register(Builder {
        name: "builder-a".to_string(),
        display_name: "Builder A — Full Codebase Review".to_string(),
        execution: BuilderExecutionPolicy {
            preferred_class: "review".to_string(),
            preferred_engine: "openai".to_string(),
            fallback_engines: vec![],
            effort: "high".to_string(),
            default_model: "GPT-5.5".to_string(),
            review_requirements: "human".to_string(),
            memory_defaults: "project:shared".to_string(),
        },
    });

    // Builder B: Quick explain, lighter
    registry.register(Builder {
        name: "builder-b".to_string(),
        display_name: "Builder B — Quick Code Explain".to_string(),
        execution: BuilderExecutionPolicy {
            preferred_class: "general".to_string(),
            preferred_engine: "openai".to_string(),
            fallback_engines: vec![],
            effort: "low".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            review_requirements: "none".to_string(),
            memory_defaults: "pane".to_string(),
        },
    });

    // Builder C: Grok primary with OpenAI fallback (as per previous phases)
    registry.register(Builder {
        name: "builder-c".to_string(),
        display_name: "Builder C — Grok Build + OpenAI Fallback".to_string(),
        execution: BuilderExecutionPolicy {
            preferred_class: "implementation".to_string(),
            preferred_engine: "grok".to_string(),
            fallback_engines: vec!["openai".to_string()],
            effort: "high".to_string(),
            default_model: "grok-build".to_string(),
            review_requirements: "sequential".to_string(),
            memory_defaults: "project:shared".to_string(),
        },
    });

    // Note: In full impl, these would be loaded from .builderboard/builders/*.yaml
    // using serde_yaml on BUILDER.yaml format matching the structs.
}

/// Example of what BUILDER.yaml would look like (for docs / future loader)
pub const BUILDER_A_EXAMPLE: &str = r#"
name: builder-a
display_name: "Builder A — Full Codebase Review"
execution:
  preferred_engine: openai
  effort: high
  default_model: GPT-5.5
  review_requirements: human
  memory_defaults: project:shared
"#;
