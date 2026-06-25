use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

fn detect_pm(cwd: Option<&std::path::Path>) -> &'static str {
    let marker_files = [
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("Cargo.toml", "cargo"),
        ("go.mod", "go"),
        ("Gemfile", "bundle"),
        ("requirements.txt", "pip"),
        ("Pipfile", "pipenv"),
        ("Cargo.lock", "cargo"),
        ("composer.json", "composer"),
    ];

    if let Some(cwd) = cwd {
        for (marker, pm) in &marker_files {
            if cwd.join(marker).exists() {
                return pm;
            }
        }
    }
    if std::env::current_dir()
        .map(|d| d.join("package-lock.json").exists())
        .unwrap_or(false)
    {
        "npm"
    } else if std::env::current_dir()
        .map(|d| d.join("Cargo.lock").exists())
        .unwrap_or(false)
    {
        "cargo"
    } else {
        "npm"
    }
}

fn run_pm_command(
    ctx: &ToolContext,
    cmd_str: String,
    exec_id: &str,
    on_event: &dyn Fn(ExecutionEvent),
) -> Result<String, String> {
    use std::process::Command;

    on_event(ExecutionEvent::ToolOutput {
        tool_id: "package".to_string(),
        execution_id: exec_id.to_string(),
        channel: "stdout".to_string(),
        content: format!("$ {}", cmd_str),
    });

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &cmd_str])
            .current_dir(
                ctx.cwd
                    .as_ref()
                    .unwrap_or(&std::env::current_dir().unwrap()),
            )
            .output()
            .map_err(|e| format!("Failed to run package command: {}", e))?
    } else {
        Command::new("sh")
            .args(["-c", &cmd_str])
            .current_dir(
                ctx.cwd
                    .as_ref()
                    .unwrap_or(&std::env::current_dir().unwrap()),
            )
            .output()
            .map_err(|e| format!("Failed to run package command: {}", e))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stdout.is_empty() {
        on_event(ExecutionEvent::ToolOutput {
            tool_id: "package".to_string(),
            execution_id: exec_id.to_string(),
            channel: "stdout".to_string(),
            content: stdout.clone(),
        });
    }
    if !stderr.is_empty() {
        on_event(ExecutionEvent::ToolOutput {
            tool_id: "package".to_string(),
            execution_id: exec_id.to_string(),
            channel: "stderr".to_string(),
            content: stderr.clone(),
        });
    }

    if !output.status.success() {
        return Err(format!("Package command failed: {}", stderr));
    }

    Ok(stdout)
}

// ---------------------------------------------------------------------------
// InstallTool
// ---------------------------------------------------------------------------

pub struct InstallTool;

impl Tool for InstallTool {
    fn id(&self) -> ToolId {
        ToolId("package.install")
    }
    fn display_name(&self) -> String {
        "Install Package".to_string()
    }
    fn description(&self) -> String {
        "Install a package using the detected package manager.".to_string()
    }
    fn category_name(&self) -> String {
        "packages".to_string()
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
        vec![ToolPermission::Packages]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let name = args.get("name").and_then(|v| v.as_str());
        if name.is_none() || name.unwrap().is_empty() {
            return Err("Missing required argument: 'name'".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_packages, "packages", &|e| on_event(e))?;
        helpers::check_permission(&ctx, ctx.allow_shell, "shell", &|e| on_event(e))?;

        let name = args["name"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "package.install".to_string(),
            execution_id: exec_id.clone(),
            args: format!("install {}", name),
        });

        let pm = detect_pm(ctx.cwd.as_deref());
        let cmd = match pm {
            "bun" => format!("bun add {}", name),
            "pnpm" => format!("pnpm add {}", name),
            "yarn" => format!("yarn add {}", name),
            "cargo" => format!("cargo add {}", name),
            "go" => format!("go get {}", name),
            "pip" => format!("pip install {}", name),
            "bundle" => format!("bundle add {}", name),
            "pipenv" => format!("pipenv install {}", name),
            "composer" => format!("composer require {}", name),
            _ => format!("npm install {}", name),
        };

        let stdout = run_pm_command(&ctx, cmd, &exec_id, on_event)?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "package.install".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Installed package '{}'", name)),
        });

        helpers::emit_timeline(
            &exec_id,
            "package.install",
            "completed",
            &format!("Installed package '{}'", name),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "package.install".to_string(),
            summary: format!("Installed package '{}' via {}", name, pm),
            details: Some(stdout.clone()),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "package.install".to_string(),
            execution_id: exec_id.clone(),
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "package.installed".to_string(),
            summary: format!("Installed package: {}", name),
            content: Some(stdout),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("Installed package '{}'", name),
                String::new(),
                format!("Installed {}", name),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// UninstallTool
// ---------------------------------------------------------------------------

pub struct UninstallTool;

impl Tool for UninstallTool {
    fn id(&self) -> ToolId {
        ToolId("package.uninstall")
    }
    fn display_name(&self) -> String {
        "Uninstall Package".to_string()
    }
    fn description(&self) -> String {
        "Uninstall a package using the detected package manager.".to_string()
    }
    fn category_name(&self) -> String {
        "packages".to_string()
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
        vec![ToolPermission::Packages]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let name = args.get("name").and_then(|v| v.as_str());
        if name.is_none() || name.unwrap().is_empty() {
            return Err("Missing required argument: 'name'".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_packages, "packages", &|e| on_event(e))?;
        helpers::check_permission(&ctx, ctx.allow_shell, "shell", &|e| on_event(e))?;

        let name = args["name"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "package.uninstall".to_string(),
            execution_id: exec_id.clone(),
            args: format!("uninstall {}", name),
        });

        let pm = detect_pm(ctx.cwd.as_deref());
        let cmd = match pm {
            "bun" => format!("bun remove {}", name),
            "pnpm" => format!("pnpm remove {}", name),
            "yarn" => format!("yarn remove {}", name),
            "cargo" => format!("cargo remove {}", name),
            "pip" => format!("pip uninstall -y {}", name),
            _ => format!("npm uninstall {}", name),
        };

        let stdout = run_pm_command(&ctx, cmd, &exec_id, on_event)?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "package.uninstall".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Uninstalled package '{}'", name)),
        });

        helpers::emit_timeline(
            &exec_id,
            "package.uninstall",
            "completed",
            &format!("Uninstalled package '{}'", name),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "package.uninstall".to_string(),
            summary: format!("Uninstalled package '{}' via {}", name, pm),
            details: Some(stdout.clone()),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "package.uninstall".to_string(),
            execution_id: exec_id,
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "package.uninstalled".to_string(),
            summary: format!("Uninstalled package: {}", name),
            content: Some(stdout),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("Uninstalled package '{}'", name),
                String::new(),
                format!("Uninstalled {}", name),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// ListTool
// ---------------------------------------------------------------------------

pub struct ListTool;

impl Tool for ListTool {
    fn id(&self) -> ToolId {
        ToolId("package.list")
    }
    fn display_name(&self) -> String {
        "List Packages".to_string()
    }
    fn description(&self) -> String {
        "List installed packages using the detected package manager.".to_string()
    }
    fn category_name(&self) -> String {
        "packages".to_string()
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
        vec![ToolPermission::ReadFiles]
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
        helpers::check_permission(&ctx, ctx.allow_read, "read_files", &|e| on_event(e))?;
        helpers::check_permission(&ctx, ctx.allow_shell, "shell", &|e| on_event(e))?;

        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "package.list".to_string(),
            execution_id: exec_id.clone(),
            args: "list packages".to_string(),
        });

        let pm = detect_pm(ctx.cwd.as_deref());
        let cmd = match pm {
            "bun" => "bun pm ls".to_string(),
            "pnpm" => "pnpm ls --depth=0".to_string(),
            "yarn" => "yarn list --depth=0".to_string(),
            "cargo" => "cargo install --list".to_string(),
            "pip" => "pip list".to_string(),
            _ => "npm ls --depth=0".to_string(),
        };

        let stdout = run_pm_command(&ctx, cmd, &exec_id, on_event)?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "package.list".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Listed packages via {}", pm)),
        });

        helpers::emit_timeline(
            &exec_id,
            "package.list",
            "completed",
            &format!("Listed packages via {}", pm),
            &|e| on_event(e),
        );

        let stdout_clone = stdout.clone();
        let artifact = ToolArtifact {
            artifact_type: "package.list".to_string(),
            summary: format!("Package listing via {}", pm),
            content: Some(stdout),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                stdout_clone,
                String::new(),
                format!("Listed packages via {}", pm),
            ),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}
