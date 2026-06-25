//! Tool permission model.
//!
//! Every tool declares its required permissions.
//! ExecutionPolicy gates which permissions are allowed at runtime.

use serde::{Deserialize, Serialize};

/// Named permission categories that tools can require.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPermission {
    ReadFiles,
    WriteFiles,
    DeleteFiles,
    Shell,
    Network,
    Git,
    Packages,
    Processes,
}

/// Whether a permission is required, optional, or never.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Tool always needs this permission.
    Required,
    /// Tool may use this permission if granted.
    Optional,
    /// Tool never uses this permission.
    Denied,
}

impl ToolPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolPermission::ReadFiles => "read_files",
            ToolPermission::WriteFiles => "write_files",
            ToolPermission::DeleteFiles => "delete_files",
            ToolPermission::Shell => "shell",
            ToolPermission::Network => "network",
            ToolPermission::Git => "git",
            ToolPermission::Packages => "packages",
            ToolPermission::Processes => "processes",
        }
    }
}
