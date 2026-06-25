use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

// ---------------------------------------------------------------------------
// HealthTool
// ---------------------------------------------------------------------------

pub struct HealthTool;

impl Tool for HealthTool {
    fn id(&self) -> ToolId {
        ToolId("diagnostics.health")
    }
    fn display_name(&self) -> String {
        "Health Check".to_string()
    }
    fn description(&self) -> String {
        "Check the health and availability of external tools and dependencies.".to_string()
    }
    fn category_name(&self) -> String {
        "system".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::Testing,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![]
    }

    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        _args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "diagnostics.health".to_string(),
            execution_id: exec_id.clone(),
            args: "check health".to_string(),
        });

        let checks = vec![
            ("sh", check_cmd("sh", &["--version"])),
            ("git", check_cmd("git", &["--version"])),
            ("node", check_cmd("node", &["--version"])),
            ("npm", check_cmd("npm", &["--version"])),
            ("rg", check_cmd("rg", &["--version"])),
            ("fd", check_cmd("fd", &["--version"])),
        ];

        let mut report = String::from("Tool Health Check\n=================\n\n");
        let mut available = 0u32;
        let mut missing = 0u32;

        for (name, ok) in &checks {
            let status = if *ok { "available" } else { "not found" };
            if *ok {
                available += 1;
            } else {
                missing += 1;
            }
            report.push_str(&format!("  {}  {}\n", status, name));
        }

        report.push_str(&format!("\n{} available, {} missing\n", available, missing));

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "diagnostics.health".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: report.clone(),
        });

        let summary = format!("{} available, {} missing", available, missing);
        on_event(ExecutionEvent::ToolFinished {
            tool_id: "diagnostics.health".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(
            &exec_id,
            "diagnostics.health",
            "completed",
            &summary,
            &|e| on_event(e),
        );

        let artifact = ToolArtifact {
            artifact_type: "diagnostics.health".to_string(),
            summary: summary.clone(),
            content: Some(report),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: missing == 0,
            exit_code: Some(if missing == 0 { 0 } else { 1 }),
            output: ToolOutput::new(summary.clone(), String::new(), summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

fn check_cmd(name: &str, args: &[&str]) -> bool {
    std::process::Command::new(name)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// EnvTool
// ---------------------------------------------------------------------------

pub struct EnvTool;

impl Tool for EnvTool {
    fn id(&self) -> ToolId {
        ToolId("diagnostics.env")
    }
    fn display_name(&self) -> String {
        "Environment Info".to_string()
    }
    fn description(&self) -> String {
        "Show environment information (OS, Rust version, etc.).".to_string()
    }
    fn category_name(&self) -> String {
        "system".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::Testing,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![]
    }

    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        _args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "diagnostics.env".to_string(),
            execution_id: exec_id.clone(),
            args: "show environment".to_string(),
        });

        let mut info = String::new();
        info.push_str(&format!("OS:        {}\n", std::env::consts::OS));
        info.push_str(&format!("Arch:      {}\n", std::env::consts::ARCH));
        info.push_str(&format!("Rust:      {}\n", rustc_version()));
        info.push_str(&format!(
            "CWD:       {}\n",
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ));
        info.push_str(&format!("Cores:     {}\n", num_cpus()));
        info.push_str(&format!("PID:       {}\n", std::process::id()));

        let cwd_str = ctx
            .cwd
            .as_ref()
            .or(ctx.project_root.as_ref())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        info.push_str(&format!("Tool CWD:  {}\n", cwd_str));
        info.push_str(&format!("Exec ID:   {}\n", exec_id));

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "diagnostics.env".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: info.clone(),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "diagnostics.env".to_string(),
            execution_id: exec_id.clone(),
            summary: Some("Environment info collected".to_string()),
        });

        helpers::emit_timeline(
            &exec_id,
            "diagnostics.env",
            "completed",
            "Environment info collected",
            &|e| on_event(e),
        );

        let info_clone = info.clone();
        let artifact = ToolArtifact {
            artifact_type: "diagnostics.env".to_string(),
            summary: "Environment information".to_string(),
            content: Some(info),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                info_clone,
                String::new(),
                "Environment info collected".to_string(),
            ),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
