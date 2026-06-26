use std::sync::Arc;

use serde_json::json;

use crate::execution::context::ExecutionPolicy;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::registry::ToolRegistry;
use crate::execution::tools::traits::Tool;
use crate::models::{Conversation, MessageRole};

/// Reusable capability profiles used to minimize the tool surface per mission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityProfile {
    ReasoningOnly,
    ReadOnly,
    Review,
    Implementation,
    Diagnostics,
}

impl CapabilityProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityProfile::ReasoningOnly => "reasoning_only",
            CapabilityProfile::ReadOnly => "read_only",
            CapabilityProfile::Review => "review",
            CapabilityProfile::Implementation => "implementation",
            CapabilityProfile::Diagnostics => "diagnostics",
        }
    }

    pub fn tool_ids(&self) -> &'static [&'static str] {
        match self {
            CapabilityProfile::ReasoningOnly => &[],
            CapabilityProfile::ReadOnly => &["filesystem.read", "search.grep", "search.glob"],
            CapabilityProfile::Review => &[
                "filesystem.read",
                "search.grep",
                "search.glob",
                "git.status",
                "git.diff",
                "diagnostics.health",
            ],
            CapabilityProfile::Implementation => &[
                "filesystem.read",
                "filesystem.write",
                "filesystem.edit",
                "directory.list",
                "directory.create",
                "search.grep",
                "search.glob",
                "git.status",
                "git.diff",
                "git.log",
                "shell",
                "package.list",
                "package.install",
                "diagnostics.health",
                "diagnostics.env",
            ],
            CapabilityProfile::Diagnostics => &[
                "filesystem.read",
                "search.grep",
                "search.glob",
                "shell",
                "diagnostics.health",
                "diagnostics.env",
            ],
        }
    }
}

/// Mission classification result for diagnostics and profile resolution.
#[derive(Clone, Debug)]
pub struct MissionClassification {
    pub mission: String,
    pub class: ExecutionClass,
    pub profile: CapabilityProfile,
    pub reason: String,
}

impl MissionClassification {
    fn new(mission: &str, class: ExecutionClass, profile: CapabilityProfile, reason: &str) -> Self {
        Self {
            mission: mission.to_string(),
            class,
            profile,
            reason: reason.to_string(),
        }
    }
}

/// Audit report for capability resolution pipeline.
#[derive(Clone, Debug)]
pub struct AuditReport {
    pub registered_count: usize,
    pub allowed_count: usize,
    pub blocked_count: usize,
    pub tool_ids: Vec<String>,
    pub allowed_tool_ids: Vec<String>,
    pub blocked_tool_ids: Vec<String>,
    pub policy: ExecutionPolicy,
}

impl AuditReport {
    pub fn summary(&self) -> String {
        format!(
            "Capability Audit: {} registered, {} allowed, {} blocked by policy",
            self.registered_count, self.allowed_count, self.blocked_count
        )
    }

    pub fn detailed(&self) -> String {
        let mut lines = vec![self.summary()];
        lines.push(format!("  Policy: {:?}", self.policy));
        lines.push("  Allowed tools:".to_string());
        for id in &self.allowed_tool_ids {
            lines.push(format!("    ✅ {}", id));
        }
        lines.push("  Blocked by policy:".to_string());
        for id in &self.blocked_tool_ids {
            lines.push(format!("    ❌ {} (check policy permissions)", id));
        }
        lines.join("\n")
    }
}

/// Run a full audit of the capability resolution pipeline.
pub fn audit_capabilities(policy: &ExecutionPolicy, registry: &ToolRegistry) -> AuditReport {
    let all_tools = registry.list();
    let tool_ids: Vec<String> = all_tools.iter().map(|t| t.id().to_string()).collect();

    let allowed: Vec<_> = all_tools
        .iter()
        .filter(|t| {
            t.permissions()
                .iter()
                .all(|p| is_permission_allowed(p, policy))
        })
        .collect();

    let allowed_tool_ids: Vec<String> = allowed.iter().map(|t| t.id().to_string()).collect();
    let blocked_tool_ids: Vec<String> = tool_ids
        .iter()
        .filter(|id| !allowed_tool_ids.contains(id))
        .cloned()
        .collect();

    AuditReport {
        registered_count: tool_ids.len(),
        allowed_count: allowed_tool_ids.len(),
        blocked_count: blocked_tool_ids.len(),
        tool_ids,
        allowed_tool_ids,
        blocked_tool_ids,
        policy: policy.clone(),
    }
}

/// Classify a conversation into a mission and minimal capability profile.
pub fn classify_mission(conversation: &Conversation) -> MissionClassification {
    let prompt = conversation
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.as_str())
        .unwrap_or_default();

    classify_prompt(prompt)
}

/// Classify a plain prompt. Kept deterministic and conservative.
pub fn classify_prompt(prompt: &str) -> MissionClassification {
    let text = prompt.to_ascii_lowercase();

    if contains_any(
        &text,
        &[
            "cargo check",
            "cargo test",
            "npm run",
            "run tests",
            "run checks",
            "validate",
            "test suite",
        ],
    ) {
        return MissionClassification::new(
            "testing/validation",
            ExecutionClass::Testing,
            CapabilityProfile::Diagnostics,
            "Prompt asks to run validation or tests.",
        );
    }

    if contains_any(
        &text,
        &[
            "create ",
            "write ",
            "edit ",
            "modify ",
            "implement",
            "fix ",
            "delete ",
            "add ",
            "rename ",
        ],
    ) && !contains_any(
        &text,
        &[
            "write an executive summary",
            "write executive summary",
            "write a summary",
            "write summary",
        ],
    ) {
        return MissionClassification::new(
            "implementation",
            ExecutionClass::Implementation,
            CapabilityProfile::Implementation,
            "Prompt asks to create or change project artifacts.",
        );
    }

    if contains_any(
        &text,
        &[
            "review",
            "audit",
            "inspect",
            "risk",
            "regression",
            "code review",
        ],
    ) {
        return MissionClassification::new(
            "review",
            ExecutionClass::Review,
            CapabilityProfile::Review,
            "Prompt asks for review or audit work.",
        );
    }

    if contains_any(
        &text,
        &[
            "explain this file",
            "explain the file",
            "document",
            "documentation",
            "summarize this file",
            "describe this file",
        ],
    ) {
        return MissionClassification::new(
            "documentation",
            ExecutionClass::Documentation,
            CapabilityProfile::ReadOnly,
            "Prompt asks for explanation or documentation.",
        );
    }

    if contains_any(
        &text,
        &[
            "executive summary",
            "architecture",
            "architectural",
            "design",
            "strategy",
            "plan",
            "roadmap",
        ],
    ) {
        return MissionClassification::new(
            "architecture/planning",
            ExecutionClass::Architecture,
            CapabilityProfile::ReadOnly,
            "Prompt asks for architecture, planning, or summary reasoning.",
        );
    }

    if contains_any(&text, &["research", "find", "search"]) {
        return MissionClassification::new(
            "research",
            ExecutionClass::Research,
            CapabilityProfile::ReadOnly,
            "Prompt asks for research or discovery.",
        );
    }

    MissionClassification::new(
        "general",
        ExecutionClass::General,
        CapabilityProfile::ReasoningOnly,
        "No tool-requiring mission signal detected.",
    )
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

/// Resolve all tools a builder is permitted to use given an ExecutionPolicy.
pub fn resolve_allowed_tools<'a>(
    policy: &ExecutionPolicy,
    registry: &'a ToolRegistry,
) -> Vec<Arc<dyn Tool>> {
    registry
        .list()
        .into_iter()
        .filter(|tool| {
            tool.permissions()
                .iter()
                .all(|perm| is_permission_allowed(perm, policy))
        })
        .collect()
}

/// Resolve the smallest profile-approved tool surface for a mission.
pub fn resolve_profile_tools<'a>(
    policy: &ExecutionPolicy,
    registry: &'a ToolRegistry,
    profile: &CapabilityProfile,
) -> Vec<Arc<dyn Tool>> {
    let profile_ids = profile.tool_ids();
    if profile_ids.is_empty() {
        return vec![];
    }

    registry
        .list()
        .into_iter()
        .filter(|tool| profile_ids.contains(&tool.id().as_str()))
        .filter(|tool| {
            tool.permissions()
                .iter()
                .all(|perm| is_permission_allowed(perm, policy))
        })
        .collect()
}

/// Public check: whether a single permission is allowed by the policy.
pub fn tool_permission_allowed(perm: &ToolPermission, policy: &ExecutionPolicy) -> bool {
    is_permission_allowed(perm, policy)
}

/// Check whether a single permission is allowed by the policy.
fn is_permission_allowed(perm: &ToolPermission, policy: &ExecutionPolicy) -> bool {
    match perm {
        ToolPermission::ReadFiles => policy.allow_read,
        ToolPermission::WriteFiles => policy.allow_write,
        ToolPermission::DeleteFiles => policy.allow_delete,
        ToolPermission::Shell => policy.allow_shell,
        ToolPermission::Network => policy.allow_network,
        ToolPermission::Git => policy.allow_git,
        ToolPermission::Packages => policy.allow_packages,
        ToolPermission::Processes => policy.allow_processes,
    }
}

/// Get the reason a tool is blocked by policy, if any.
fn tool_blocked_reason(tool: &dyn Tool, policy: &ExecutionPolicy) -> Option<String> {
    let blocked: Vec<String> = tool
        .permissions()
        .iter()
        .filter(|p| !is_permission_allowed(p, policy))
        .map(|p| p.as_str().to_string())
        .collect();
    if blocked.is_empty() {
        None
    } else {
        Some(blocked.join(", "))
    }
}

/// Usage examples for each tool, used in the advertisement and API responses.
pub fn tool_usage_examples(tool_id: &str) -> Vec<String> {
    match tool_id {
        "shell" => vec![
            r#"{"command": "ls -la"}"#.into(),
            r#"{"command": "cargo build", "timeout": 60000}"#.into(),
        ],
        "filesystem.read" => vec![
            r#"{"path": "src/main.rs"}"#.into(),
            r#"{"path": "README.md"}"#.into(),
        ],
        "filesystem.write" => vec![r#"{"path": "src/new.rs", "content": "fn main() {}"}"#.into()],
        "filesystem.edit" => vec![
            r#"{"path": "src/lib.rs", "old_string": "old_fn()", "new_string": "new_fn()"}"#.into(),
        ],
        "filesystem.delete" => vec![r#"{"path": "temp.txt"}"#.into()],
        "directory.list" => vec![r#"{"path": "."}"#.into(), r#"{"path": "src"}"#.into()],
        "directory.create" => vec![r#"{"path": "src/components"}"#.into()],
        "package.install" => vec![
            r#"{"name": "typescript"}"#.into(),
            r#"{"name": "serde"}"#.into(),
        ],
        "package.uninstall" => vec![r#"{"name": "lodash"}"#.into()],
        "package.list" => vec![r#"{}"#.into()],
        "git.status" => vec![r#"{}"#.into()],
        "git.diff" => vec![
            r#"{}"#.into(),
            r#"{"staged": true}"#.into(),
            r#"{"staged": true, "path": "src/main.rs"}"#.into(),
        ],
        "git.commit" => vec![r#"{"message": "fix: resolve type error"}"#.into()],
        "git.log" => vec![r#"{}"#.into(), r#"{"max_count": 5}"#.into()],
        "process.list" => vec![r#"{}"#.into(), r#"{"filter": "node"}"#.into()],
        "process.kill" => vec![
            r#"{"pid": 1234}"#.into(),
            r#"{"pid": 1234, "signal": "SIGKILL"}"#.into(),
        ],
        "search.grep" => vec![
            r#"{"pattern": "TODO"}"#.into(),
            r#"{"pattern": "fn main", "include": "rs"}"#.into(),
        ],
        "search.glob" => vec![
            r#"{"pattern": "*.rs"}"#.into(),
            r#"{"pattern": "**/*.ts", "max_results": 20}"#.into(),
        ],
        "diagnostics.health" => vec![r#"{}"#.into()],
        "diagnostics.env" => vec![r#"{}"#.into()],
        _ => vec![r#"{}"#.into()],
    }
}

/// Build a system-prompt fragment advertising the available tools.
/// Includes structured JSON schemas so the LLM can invoke them.
pub fn build_tool_advertisement(tools: &[Arc<dyn Tool>]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "You have access to the following tools. When you need to read files, search code, run commands, or perform any operation on this project, use these tools instead of hallucinating or guessing.".to_string(),
        String::new(),
        "## Available Tools".to_string(),
        String::new(),
    ];

    for tool in tools {
        let desc = tool.describe();
        let schema = tool_input_schema(tool.id().as_str());
        let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_default();
        let examples = tool_usage_examples(tool.id().as_str());

        lines.push(format!("### {} (`{}`)", desc.display_name, desc.id));
        lines.push(format!("- **Description**: {}", desc.description));
        lines.push(format!("- **Category**: {}", desc.category));
        lines.push(format!(
            "- **Permissions required**: {}",
            desc.permissions.join(", ")
        ));
        lines.push("- **Input schema**:".to_string());
        for line in schema_json.lines() {
            lines.push(format!("  {}", line));
        }
        if !examples.is_empty() {
            lines.push("- **Examples**:".to_string());
            for ex in &examples {
                lines.push(format!("  `{}`", ex));
            }
        }
        lines.push(String::new());
    }

    lines.push("## How to invoke a tool".to_string());
    lines.push(String::new());
    lines.push("When you need to use a tool, embed a tool call block in your response. The block must EXACTLY match this format:".to_string());
    lines.push(String::new());
    lines.push("```tool_call".to_string());
    lines.push(r#"{"tool": "filesystem.read", "arguments": {"path": "src/main.rs"}}"#.to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("Rules:".to_string());
    lines.push("- Use ````tool_call```` as the fence — NOT ` ``` ` or ` ```json `.".to_string());
    lines.push("- The JSON must be valid, single-line or pretty-printed.".to_string());
    lines.push("- Include BOTH `\"tool\"` and `\"arguments\"` keys.".to_string());
    lines.push("- Do NOT add text, comments, or explanations inside the fence.".to_string());
    lines.push("- You may include natural language BEFORE and AFTER the fence.".to_string());
    lines
        .push("- You may include MULTIPLE fences in one response to chain tool calls.".to_string());
    lines.push(String::new());
    lines.push(
        "After a tool executes, you will receive the result and can continue your response."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("IMPORTANT: Only use tools that are listed above. Do not simulate tool output. Always wait for actual results before responding to the user.".to_string());

    lines.join("\n")
}

/// Build a comprehensive advertisement that includes both available and unavailable tools.
/// Unavailable tools show why they are blocked.
pub fn build_comprehensive_tool_advertisement(
    registry: &ToolRegistry,
    policy: &ExecutionPolicy,
) -> String {
    let all_tools = registry.list();

    if all_tools.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "You have access to the following tools. When you need to read files, search code, run commands, or perform any operation on this project, use these tools instead of hallucinating or guessing.".to_string(),
        String::new(),
        "## Available Tools".to_string(),
        String::new(),
    ];

    for tool in &all_tools {
        let blocked_reason = tool_blocked_reason(tool.as_ref(), policy);
        let desc = tool.describe();
        let is_available = blocked_reason.is_none();

        if is_available {
            let schema = tool_input_schema(tool.id().as_str());
            let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_default();
            let examples = tool_usage_examples(tool.id().as_str());

            lines.push(format!("### {} (`{}`)", desc.display_name, desc.id));
            lines.push(format!("- **Description**: {}", desc.description));
            lines.push(format!("- **Category**: {}", desc.category));
            lines.push("- **Input schema**:".to_string());
            for line in schema_json.lines() {
                lines.push(format!("  {}", line));
            }
            if !examples.is_empty() {
                lines.push("- **Examples**:".to_string());
                for ex in &examples {
                    lines.push(format!("  `{}`", ex));
                }
            }
            lines.push(String::new());
        }
    }

    let unavailable: Vec<_> = all_tools
        .iter()
        .filter(|t| tool_blocked_reason(t.as_ref(), policy).is_some())
        .collect();

    if !unavailable.is_empty() {
        lines.push("## Unavailable Tools (permission denied)".to_string());
        lines.push(String::new());
        for tool in &unavailable {
            let reason = tool_blocked_reason(tool.as_ref(), policy).unwrap_or_default();
            let desc = tool.describe();
            lines.push(format!(
                "- **{}** (`{}`): requires `{}` which is denied by the current execution policy.",
                desc.display_name, desc.id, reason
            ));
        }
        lines.push(String::new());
    }

    lines.push("## How to invoke a tool".to_string());
    lines.push(String::new());
    lines.push("When you need to use a tool, embed a tool call block in your response. The block must EXACTLY match this format:".to_string());
    lines.push(String::new());
    lines.push("```tool_call".to_string());
    lines.push(r#"{"tool": "filesystem.read", "arguments": {"path": "src/main.rs"}}"#.to_string());
    lines.push("```".to_string());
    lines.push(String::new());
    lines.push("Rules:".to_string());
    lines.push("- Use ````tool_call```` as the fence — NOT ` ``` ` or ` ```json `.".to_string());
    lines.push("- The JSON must be valid, single-line or pretty-printed.".to_string());
    lines.push("- Include BOTH `\"tool\"` and `\"arguments\"` keys.".to_string());
    lines.push("- Do NOT add text, comments, or explanations inside the fence.".to_string());
    lines.push("- You may include natural language BEFORE and AFTER the fence.".to_string());
    lines
        .push("- You may include MULTIPLE fences in one response to chain tool calls.".to_string());
    lines.push(String::new());
    lines.push(
        "After a tool executes, you will receive the result and can continue your response."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("IMPORTANT: Only use tools that are listed above. Do not simulate tool output. Always wait for actual results before responding to the user.".to_string());

    lines.join("\n")
}

/// Parse tool calls from an LLM response text.
///
/// Accepts multiple tool call formats for robustness:
/// - ` ```tool_call\n{"tool": "...", "arguments": {...}}\n``` ` (canonical — newline after fence)
/// - ` ```tool_call{"tool":"...","arguments":{}}``` ` (compact — no newline)
/// - ` ```\n{"tool": "...", "arguments": {...}}\n``` ` (generic fence)
/// - ` ```json\n{"tool": "...", "arguments": {...}}\n``` ` (json fence)
///
/// Strategy: find any triple-backtick fence, look for opening `{`,
/// use brace-depth matching to extract JSON, then parse. This handles
/// all fence labels (tool_call, json, none) and both multi-line and
/// single-line (compact) layouts.
///
/// Returns a list of (tool_name, arguments_json) tuples.
pub fn parse_tool_calls(response: &str) -> Vec<(String, serde_json::Value)> {
    let mut calls = Vec::new();
    let mut search_start = 0;

    loop {
        // Find any ``` that could be a tool-call fence
        let fence_start = match response[search_start..].find("```") {
            Some(idx) => search_start + idx,
            None => break,
        };

        let after_fence = fence_start + 3;

        // Look for opening { after this fence (skip past any label)
        let content_pos = match response[after_fence..].find('{') {
            Some(idx) => after_fence + idx,
            None => {
                // No JSON here — skip past the fence and continue
                search_start = after_fence;
                continue;
            }
        };

        // Find the matching closing } using brace-depth tracking
        let json_end = match find_matching_brace(&response[content_pos..]) {
            Some(end) => content_pos + end,
            None => {
                // Unmatched brace — skip past the fence
                search_start = after_fence;
                continue;
            }
        };

        let json_str = &response[content_pos..json_end];
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            let tool = val.get("tool").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = val.get("arguments").cloned().unwrap_or(json!({}));
            if !tool.is_empty() {
                calls.push((tool.to_string(), arguments));
            }
        }

        // Advance past this entire fence block
        if let Some(close_idx) = response[json_end..].find("```") {
            search_start = json_end + close_idx + 3;
        } else {
            search_start = json_end;
        }
    }

    calls
}

/// Find the position after the matching `}` for the first `{` in `text`.
/// Handles nested braces and string contents with proper escape handling.
fn find_matching_brace(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'{') {
        return None;
    }
    enum State {
        Normal,
        InString,
        Escaped,
    }
    let mut state = State::Normal;
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate() {
        match state {
            State::Normal => match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1); // past the matching }
                    }
                }
                b'"' => state = State::InString,
                _ => {}
            },
            State::InString => match b {
                b'\\' => state = State::Escaped,
                b'"' => state = State::Normal,
                _ => {}
            },
            State::Escaped => {
                state = State::InString; // skip escaped char
            }
        }
    }
    None // unbalanced braces
}

/// Generate a JSON Schema describing the input arguments for a given tool.
pub fn tool_input_schema(tool_id: &str) -> serde_json::Value {
    match tool_id {
        "shell" => json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "cwd": { "type": "string", "description": "Working directory (optional)" },
                "timeout": { "type": "integer", "description": "Timeout in milliseconds (optional)" }
            },
            "required": ["command"]
        }),
        "filesystem.read" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to read" }
            },
            "required": ["path"]
        }),
        "filesystem.write" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        }),
        "filesystem.edit" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "old_string": { "type": "string", "description": "Text to replace" },
                "new_string": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "old_string", "new_string"]
        }),
        "filesystem.delete" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to delete" }
            },
            "required": ["path"]
        }),
        "directory.list" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path to list" }
            },
            "required": ["path"]
        }),
        "directory.create" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path to create" }
            },
            "required": ["path"]
        }),
        "package.install" => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Package name to install" }
            },
            "required": ["name"]
        }),
        "package.uninstall" => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Package name to uninstall" }
            },
            "required": ["name"]
        }),
        "package.list" => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        "git.status" => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        "git.diff" => json!({
            "type": "object",
            "properties": {
                "staged": { "type": "boolean", "description": "Show staged changes only (optional)" },
                "path": { "type": "string", "description": "File or directory to diff (optional)" }
            },
            "required": []
        }),
        "git.commit" => json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message" }
            },
            "required": ["message"]
        }),
        "git.log" => json!({
            "type": "object",
            "properties": {
                "max_count": { "type": "integer", "description": "Maximum number of commits (optional)" }
            },
            "required": []
        }),
        "process.list" => json!({
            "type": "object",
            "properties": {
                "filter": { "type": "string", "description": "Process name filter (optional)" }
            },
            "required": []
        }),
        "process.kill" => json!({
            "type": "object",
            "properties": {
                "pid": { "type": "integer", "description": "Process ID to kill" },
                "signal": { "type": "string", "description": "Signal name: SIGTERM, SIGKILL, SIGINT, SIGHUP, SIGSTOP, SIGCONT (optional, default SIGTERM)" }
            },
            "required": ["pid"]
        }),
        "search.grep" => json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Search pattern (regex)" },
                "path": { "type": "string", "description": "Search root path (optional)" },
                "include": { "type": "string", "description": "File extension filter (optional)" },
                "max_results": { "type": "integer", "description": "Maximum results (optional, default 50)" },
                "fixed_string": { "type": "boolean", "description": "Literal string search (optional)" },
                "context": { "type": "integer", "description": "Context lines (optional)" }
            },
            "required": ["pattern"]
        }),
        "search.glob" => json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern to match" },
                "path": { "type": "string", "description": "Search root path (optional)" },
                "max_results": { "type": "integer", "description": "Maximum results (optional, default 100)" }
            },
            "required": ["pattern"]
        }),
        "diagnostics.health" => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        "diagnostics.env" => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        _ => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

/// Produce a human-readable summary of resolved capabilities for logging.
pub fn summarize_capabilities(tools: &[Arc<dyn Tool>]) -> String {
    if tools.is_empty() {
        return "No tools available (restricted by policy)".to_string();
    }

    let mut lines: Vec<String> = tools
        .iter()
        .map(|t| {
            let desc = t.describe();
            format!("  {} — {} ({})", desc.id, desc.display_name, desc.category)
        })
        .collect();
    lines.sort();
    lines.insert(0, format!("{} tool(s) available:", tools.len()));
    lines.join("\n")
}

/// Build a conversation system message with tool definitions.
/// Returns None if no tools are available.
pub fn build_tool_system_message(tools: &[Arc<dyn Tool>]) -> Option<String> {
    if tools.is_empty() {
        return None;
    }
    Some(build_tool_advertisement(tools))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::context::ExecutionPolicy;
    use crate::execution::event::ExecutionEvent;
    use crate::execution::manager::ExecutionClass;
    use crate::execution::tools::context::ToolContext;
    use crate::execution::tools::permissions::ToolPermission;
    use crate::execution::tools::registry::ToolRegistry;
    use crate::execution::tools::results::{ToolOutput, ToolResult};
    use crate::execution::tools::traits::{Tool, ToolId};
    use std::sync::Arc;

    struct MockTool;
    impl Tool for MockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.test")
        }
        fn display_name(&self) -> String {
            "Mock".to_string()
        }
        fn description(&self) -> String {
            "Test".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::ReadFiles]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            _ctx: ToolContext,
            _args: serde_json::Value,
            _on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct ShellMockTool;
    impl Tool for ShellMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.shell")
        }
        fn display_name(&self) -> String {
            "Mock Shell".to_string()
        }
        fn description(&self) -> String {
            "Test shell".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::Shell]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            _ctx: ToolContext,
            _args: serde_json::Value,
            _on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct NoPermTool;
    impl Tool for NoPermTool {
        fn id(&self) -> ToolId {
            ToolId("mock.noperm")
        }
        fn display_name(&self) -> String {
            "No Perm".to_string()
        }
        fn description(&self) -> String {
            "No perms".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            _ctx: ToolContext,
            _args: serde_json::Value,
            _on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    fn default_tool_registry_for_profiles() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(crate::execution::tools::shell::ShellTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::filesystem::ReadTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::filesystem::WriteTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::filesystem::EditTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::filesystem::DeleteTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::directory::ListTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::directory::CreateTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::package::InstallTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::package::ListTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::git::StatusTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::git::DiffTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::git::LogTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::diagnostics::HealthTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::diagnostics::EnvTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::search::GrepTool))
            .unwrap();
        registry
            .register(Arc::new(crate::execution::tools::search::GlobTool))
            .unwrap();
        registry
    }

    fn tool_ids(tools: Vec<Arc<dyn Tool>>) -> Vec<String> {
        let mut ids: Vec<String> = tools.iter().map(|tool| tool.id().to_string()).collect();
        ids.sort();
        ids
    }

    fn all_allowed_policy() -> ExecutionPolicy {
        ExecutionPolicy {
            allow_shell: true,
            allow_network: true,
            allow_read: true,
            allow_write: true,
            allow_delete: true,
            allow_git: true,
            allow_packages: true,
            allow_processes: true,
            max_tokens: None,
            timeout_ms: None,
        }
    }

    #[test]
    fn resolve_allows_tools_with_matching_permissions() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(ShellMockTool)).unwrap();

        let policy = ExecutionPolicy {
            allow_shell: true,
            allow_read: true,
            ..Default::default()
        };
        let allowed = resolve_allowed_tools(&policy, &registry);
        assert_eq!(allowed.len(), 2);

        let policy_no_shell = ExecutionPolicy {
            allow_shell: false,
            allow_read: true,
            ..Default::default()
        };
        let allowed2 = resolve_allowed_tools(&policy_no_shell, &registry);
        assert_eq!(allowed2.len(), 1);
        assert_eq!(allowed2[0].id().as_str(), "mock.test");
    }

    #[test]
    fn resolve_denies_tools_when_all_permissions_blocked() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(ShellMockTool)).unwrap();

        let policy = ExecutionPolicy {
            allow_read: false,
            allow_shell: false,
            ..Default::default()
        };
        let allowed = resolve_allowed_tools(&policy, &registry);
        assert!(allowed.is_empty());
    }

    #[test]
    fn resolve_tools_with_no_permissions_always_allowed() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(NoPermTool)).unwrap();

        let policy = ExecutionPolicy {
            allow_read: false,
            allow_shell: false,
            ..Default::default()
        };
        let allowed = resolve_allowed_tools(&policy, &registry);
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].id().as_str(), "mock.noperm");
    }

    #[test]
    fn mission_classifies_executive_summary_as_read_only_architecture() {
        let mission = classify_prompt("Write an executive summary.");
        assert_eq!(mission.class, ExecutionClass::Architecture);
        assert_eq!(mission.profile, CapabilityProfile::ReadOnly);

        let registry = default_tool_registry_for_profiles();
        let ids = tool_ids(resolve_profile_tools(
            &all_allowed_policy(),
            &registry,
            &mission.profile,
        ));
        assert_eq!(ids, vec!["filesystem.read", "search.glob", "search.grep"]);
        assert!(!ids.contains(&"filesystem.write".to_string()));
        assert!(!ids.contains(&"shell".to_string()));
        assert!(!ids.contains(&"git.commit".to_string()));
    }

    #[test]
    fn mission_classifies_review_as_review_profile() {
        let mission = classify_prompt("Review this repository.");
        assert_eq!(mission.class, ExecutionClass::Review);
        assert_eq!(mission.profile, CapabilityProfile::Review);

        let registry = default_tool_registry_for_profiles();
        let ids = tool_ids(resolve_profile_tools(
            &all_allowed_policy(),
            &registry,
            &mission.profile,
        ));
        assert!(ids.contains(&"filesystem.read".to_string()));
        assert!(ids.contains(&"git.diff".to_string()));
        assert!(ids.contains(&"search.grep".to_string()));
        assert!(!ids.contains(&"filesystem.write".to_string()));
        assert!(!ids.contains(&"git.commit".to_string()));
    }

    #[test]
    fn mission_classifies_file_creation_as_implementation_profile() {
        let mission = classify_prompt("Create docs/test.md");
        assert_eq!(mission.class, ExecutionClass::Implementation);
        assert_eq!(mission.profile, CapabilityProfile::Implementation);

        let registry = default_tool_registry_for_profiles();
        let ids = tool_ids(resolve_profile_tools(
            &all_allowed_policy(),
            &registry,
            &mission.profile,
        ));
        assert!(ids.contains(&"filesystem.write".to_string()));
        assert!(ids.contains(&"directory.create".to_string()));
        assert!(ids.contains(&"diagnostics.health".to_string()));
    }

    #[test]
    fn mission_classifies_cargo_check_as_testing_diagnostics_profile() {
        let mission = classify_prompt("Run cargo check.");
        assert_eq!(mission.class, ExecutionClass::Testing);
        assert_eq!(mission.profile, CapabilityProfile::Diagnostics);

        let registry = default_tool_registry_for_profiles();
        let ids = tool_ids(resolve_profile_tools(
            &all_allowed_policy(),
            &registry,
            &mission.profile,
        ));
        assert!(ids.contains(&"shell".to_string()));
        assert!(ids.contains(&"diagnostics.health".to_string()));
        assert!(ids.contains(&"filesystem.read".to_string()));
        assert!(!ids.contains(&"filesystem.write".to_string()));
    }

    #[test]
    fn mission_classifies_file_explanation_as_documentation_read_only() {
        let mission = classify_prompt("Explain this file.");
        assert_eq!(mission.class, ExecutionClass::Documentation);
        assert_eq!(mission.profile, CapabilityProfile::ReadOnly);

        let registry = default_tool_registry_for_profiles();
        let ids = tool_ids(resolve_profile_tools(
            &all_allowed_policy(),
            &registry,
            &mission.profile,
        ));
        assert!(ids.contains(&"filesystem.read".to_string()));
        assert!(!ids.contains(&"filesystem.write".to_string()));
        assert!(!ids.contains(&"shell".to_string()));
    }

    #[test]
    fn audit_reports_correct_counts() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(ShellMockTool)).unwrap();
        registry.register(Arc::new(NoPermTool)).unwrap();

        let policy = ExecutionPolicy {
            allow_read: true,
            allow_shell: false,
            ..Default::default()
        };
        let audit = audit_capabilities(&policy, &registry);
        assert_eq!(audit.registered_count, 3);
        assert_eq!(audit.allowed_count, 2); // MockTool (read) + NoPermTool (none)
        assert_eq!(audit.blocked_count, 1); // ShellMockTool
        assert!(audit.allowed_tool_ids.contains(&"mock.test".to_string()));
        assert!(audit.allowed_tool_ids.contains(&"mock.noperm".to_string()));
        assert!(audit.blocked_tool_ids.contains(&"mock.shell".to_string()));
    }

    #[test]
    fn build_tool_advertisement_produces_text() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool)];
        let adv = build_tool_advertisement(&tools);
        assert!(adv.contains("mock.test"));
        assert!(adv.contains("Mock"));
        assert!(adv.contains("```tool_call"));
        assert!(adv.contains("Examples"));
    }

    #[test]
    fn build_tool_advertisement_returns_empty_for_no_tools() {
        let adv = build_tool_advertisement(&[]);
        assert!(adv.is_empty());
    }

    #[test]
    fn comprehensive_advertisement_lists_unavailable() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(ShellMockTool)).unwrap();

        let policy = ExecutionPolicy {
            allow_read: true,
            allow_shell: false,
            ..Default::default()
        };
        let adv = build_comprehensive_tool_advertisement(&registry, &policy);
        assert!(adv.contains("mock.test"));
        assert!(adv.contains("mock.shell"));
        assert!(adv.contains("Unavailable Tools"));
        assert!(adv.contains("denied"));
    }

    #[test]
    fn parse_tool_calls_detects_single_call() {
        let response = r#"
I will read the file.

```tool_call
{
  "tool": "filesystem.read",
  "arguments": {"path": "src/main.rs"}
}
```

Here are the contents.
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "filesystem.read");
        assert_eq!(calls[0].1["path"], "src/main.rs");
    }

    #[test]
    fn parse_tool_calls_detects_multiple_calls() {
        let response = r#"
First:

```tool_call
{"tool": "directory.list", "arguments": {"path": "."}}
```

Then:

```tool_call
{"tool": "filesystem.read", "arguments": {"path": "README.md"}}
```
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "directory.list");
        assert_eq!(calls[1].0, "filesystem.read");
    }

    #[test]
    fn parse_tool_calls_returns_empty_when_none() {
        let response = "Hello, I am a helpful assistant.";
        let calls = parse_tool_calls(response);
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_tool_calls_handles_malformed_json() {
        let response = r#"
```tool_call
{ invalid json }
```
"#;
        let calls = parse_tool_calls(response);
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_tool_calls_ignores_missing_tool_field() {
        let response = r#"
```tool_call
{"name": "something", "arguments": {}}
```
"#;
        let calls = parse_tool_calls(response);
        assert!(calls.is_empty());
    }

    #[test]
    fn summarize_capabilities_lists_tools() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool), Arc::new(ShellMockTool)];
        let s = summarize_capabilities(&tools);
        assert!(s.contains("2 tool(s) available"));
        assert!(s.contains("mock.test"));
        assert!(s.contains("mock.shell"));
    }

    #[test]
    fn tool_input_schema_covers_all_ids() {
        let ids = [
            "shell",
            "filesystem.read",
            "filesystem.write",
            "filesystem.edit",
            "filesystem.delete",
            "directory.list",
            "directory.create",
            "package.install",
            "package.uninstall",
            "package.list",
            "git.status",
            "git.diff",
            "git.commit",
            "git.log",
            "process.list",
            "process.kill",
            "search.grep",
            "search.glob",
            "diagnostics.health",
            "diagnostics.env",
        ];
        for id in &ids {
            let schema = tool_input_schema(id);
            assert!(schema.get("type").is_some(), "Missing schema for {}", id);
        }
    }

    #[test]
    fn tool_usage_examples_covers_all_ids() {
        let ids = [
            "shell",
            "filesystem.read",
            "filesystem.write",
            "filesystem.edit",
            "filesystem.delete",
            "directory.list",
            "directory.create",
            "package.install",
            "package.uninstall",
            "package.list",
            "git.status",
            "git.diff",
            "git.commit",
            "git.log",
            "process.list",
            "process.kill",
            "search.grep",
            "search.glob",
            "diagnostics.health",
            "diagnostics.env",
        ];
        for id in &ids {
            let examples = tool_usage_examples(id);
            assert!(!examples.is_empty(), "Missing examples for {}", id);
        }
    }

    #[test]
    fn audit_summary_includes_counts() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        let policy = ExecutionPolicy {
            allow_read: true,
            ..Default::default()
        };
        let audit = audit_capabilities(&policy, &registry);
        let summary = audit.summary();
        assert!(summary.contains("registered"));
        assert!(summary.contains("allowed"));
    }

    // --- Tolerant parser tests (Phase 9A.5) ---

    #[test]
    fn parse_tool_calls_compact_no_newline() {
        // LLM emits ```tool_call immediately followed by JSON on the same line
        let response = r#"I'll check:

```tool_call{"tool": "git.status", "arguments": {}}
```

Done."#;
        let calls = parse_tool_calls(response);
        assert_eq!(
            calls.len(),
            1,
            "should parse compact format without newline after fence"
        );
        assert_eq!(calls[0].0, "git.status");
    }

    #[test]
    fn parse_tool_calls_json_fence() {
        // LLM uses ```json instead of ```tool_call
        let response = r#"
```json
{"tool": "filesystem.read", "arguments": {"path": "Cargo.toml"}}
```
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1, "should accept ```json fence");
        assert_eq!(calls[0].0, "filesystem.read");
        assert_eq!(calls[0].1["path"], "Cargo.toml");
    }

    #[test]
    fn parse_tool_calls_plain_fence() {
        // LLM uses plain ``` without any language identifier
        let response = r#"
```
{"tool": "diagnostics.health", "arguments": {}}
```
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1, "should accept plain ``` fence");
        assert_eq!(calls[0].0, "diagnostics.health");
    }

    #[test]
    fn parse_tool_calls_compact_brace_fence() {
        // LLM uses ```{ — no newline, no label, just ```{...}```
        let response = r#"
```{"tool": "directory.list", "arguments": {"path": "."}}```
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(
            calls.len(),
            1,
            "should accept triple-backtick-brace compact format"
        );
        assert_eq!(calls[0].0, "directory.list");
    }

    #[test]
    fn parse_tool_calls_skips_non_tool_fences() {
        // Regular ```rust or ```python blocks should not match
        let response = r#"
```rust
fn main() {}
```
"#;
        let calls = parse_tool_calls(response);
        assert!(calls.is_empty(), "non-tool fences should not match");
    }

    #[test]
    fn parse_tool_calls_mixed_fence_types() {
        // Multiple tool calls with different fence styles
        let response = r#"
First:

```tool_call
{"tool": "shell", "arguments": {"command": "ls"}}
```

Then:

```json
{"tool": "search.grep", "arguments": {"pattern": "TODO"}}
```

Finally compact triple-backtick-brace:

```{"tool": "git.log", "arguments": {"max_count": 3}}```
"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 3, "should parse mixed fence styles");
        assert_eq!(calls[0].0, "shell");
        assert_eq!(calls[1].0, "search.grep");
        assert_eq!(calls[2].0, "git.log");
    }

    #[test]
    fn build_tool_advertisement_includes_rules() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool)];
        let adv = build_tool_advertisement(&tools);
        assert!(
            adv.contains("```tool_call"),
            "should include fence syntax example"
        );
        assert!(
            adv.contains("Do NOT add text"),
            "should warn against comments inside fence"
        );
        assert!(adv.contains("MULTIPLE fences"), "should mention chaining");
        assert!(
            adv.contains("before responding to the user"),
            "should emphasize waiting for results"
        );
    }
}
