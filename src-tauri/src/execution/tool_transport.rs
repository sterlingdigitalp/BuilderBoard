//! Tool invocation transport models.
//!
//! ToolRegistry remains the source of truth. Transports only format resolved
//! tool metadata for engines that speak native function calling or Markdown.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::execution::capability_resolver::tool_input_schema;
use crate::execution::tools::traits::Tool;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NativeToolDefinition {
    pub name: String,
    pub tool_id: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NativeToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

impl NativeToolDefinition {
    pub fn from_tool(tool: &Arc<dyn Tool>) -> Self {
        let desc = tool.describe();
        Self {
            name: native_function_name(&desc.id),
            tool_id: desc.id.clone(),
            description: desc.description,
            parameters: tool_input_schema(&desc.id),
        }
    }
}

pub fn native_tool_definitions(tools: &[Arc<dyn Tool>]) -> Vec<NativeToolDefinition> {
    tools.iter().map(NativeToolDefinition::from_tool).collect()
}

pub fn native_tool_name_map(tools: &[NativeToolDefinition]) -> HashMap<String, String> {
    tools
        .iter()
        .map(|tool| (tool.name.clone(), tool.tool_id.clone()))
        .collect()
}

pub fn native_function_name(tool_id: &str) -> String {
    let mut name = String::with_capacity(tool_id.len());
    for ch in tool_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    if name.is_empty() {
        "tool".to_string()
    } else {
        name.chars().take(64).collect()
    }
}
