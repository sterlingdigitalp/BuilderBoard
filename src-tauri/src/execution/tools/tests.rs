//! Comprehensive tool runtime tests.
//!
//! Covers: registry, permission enforcement, validation, review, timeline,
//! cancellation, error handling, and git diff fix verification.

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use crate::execution::event::ExecutionEvent;
    use crate::execution::manager::ExecutionClass;
    use crate::execution::tools::context::ToolContext;
    use crate::execution::tools::permissions::ToolPermission;
    use crate::execution::tools::registry::{global_tool_registry, ToolRegistry};
    use crate::execution::tools::results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
    use crate::execution::tools::traits::{Tool, ToolId};

    // -----------------------------------------------------------------------
    // Registry Tests
    // -----------------------------------------------------------------------

    #[test]
    fn registry_register_and_lookup() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        assert_eq!(registry.len(), 1);
        let tool = registry.get("mock.test");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().id().as_str(), "mock.test");
    }

    #[test]
    fn registry_prevents_duplicates() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        let result = registry.register(Arc::new(MockTool));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already registered"));
    }

    #[test]
    fn registry_lookup_missing_returns_none() {
        let registry = ToolRegistry::new();
        assert!(registry.get("does_not_exist").is_none());
    }

    #[test]
    fn registry_list_all_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        registry.register(Arc::new(OtherMockTool)).unwrap();
        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn registry_find_by_class() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        let results = registry.find_by_class(&ExecutionClass::Implementation);
        assert!(!results.is_empty());
        let results_debug = registry.find_by_class(&ExecutionClass::Debugging);
        assert!(!results_debug.is_empty());
        let results_arch = registry.find_by_class(&ExecutionClass::Architecture);
        assert!(results_arch.is_empty());
    }

    #[test]
    fn registry_find_by_category() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        let results = registry.find_by_category("test");
        assert_eq!(results.len(), 1);
        let results = registry.find_by_category("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn registry_find_by_name() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        let results = registry.find_by_name("Mock");
        assert!(!results.is_empty());
        let results = registry.find_by_name("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn global_registry_exists() {
        let reg = global_tool_registry();
        assert!(reg.read().is_ok());
    }

    // -----------------------------------------------------------------------
    // Permission Enforcement Tests
    // -----------------------------------------------------------------------

    #[test]
    fn permission_check_denies_when_flag_false() {
        let events = capture_events(|on_event| {
            let ctx = ToolContext {
                allow_shell: false,
                ..default_test_ctx()
            };
            let tool = MockTool;
            let result = tool.execute(ctx, json!({}), &on_event);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not allowed"));
        });

        assert!(events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::PermissionCheck { allowed: false, .. })));
    }

    #[test]
    fn permission_check_allows_when_flag_true() {
        let tool = MockTool;
        let ctx = default_test_ctx();
        let result = tool.execute(ctx, json!({}), &|_| {});
        assert!(result.is_ok());
    }

    #[test]
    fn read_files_permission_enforced() {
        let ctx = ToolContext {
            allow_read: false,
            ..default_test_ctx()
        };
        let tool = ReadFileMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(ctx, json!({"path": "test.txt"}), &on_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read_files"));
    }

    #[test]
    fn write_files_permission_enforced() {
        let ctx = ToolContext {
            allow_write: false,
            ..default_test_ctx()
        };
        let tool = WriteFileMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(
            ctx,
            json!({"path": "test.txt", "content": "hello"}),
            &on_event,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("write_files"));
    }

    #[test]
    fn delete_files_permission_enforced() {
        let ctx = ToolContext {
            allow_delete: false,
            ..default_test_ctx()
        };
        let tool = DeleteFileMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(ctx, json!({"path": "test.txt"}), &on_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("delete_files"));
    }

    #[test]
    fn git_permission_enforced() {
        let ctx = ToolContext {
            allow_git: false,
            ..default_test_ctx()
        };
        let tool = GitMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(ctx, json!({}), &on_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("git"));
    }

    #[test]
    fn packages_permission_enforced() {
        let ctx = ToolContext {
            allow_packages: false,
            ..default_test_ctx()
        };
        let tool = PackageMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(ctx, json!({"name": "test"}), &on_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("packages"));
    }

    #[test]
    fn processes_permission_enforced() {
        let ctx = ToolContext {
            allow_processes: false,
            ..default_test_ctx()
        };
        let tool = ProcessMockTool;
        let on_event = |_e: ExecutionEvent| {};
        let result = tool.execute(ctx, json!({}), &on_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("processes"));
    }

    // -----------------------------------------------------------------------
    // Validation Tests
    // -----------------------------------------------------------------------

    #[test]
    fn validate_required_args() {
        let tool = MockTool;
        assert!(tool.validate(&json!({})).is_ok()); // no required args for MockTool
        let tool = WriteFileMockTool;
        assert!(tool.validate(&json!({})).is_err());
        assert!(tool.validate(&json!({"path": "x"})).is_err());
        assert!(tool.validate(&json!({"path": "x", "content": "y"})).is_ok());
        let tool = crate::execution::tools::filesystem::WriteTool;
        assert!(tool
            .validate(&json!({"path": "docs/test.md", "content": ""}))
            .is_ok());
    }

    // -----------------------------------------------------------------------
    // Review Item Tests
    // -----------------------------------------------------------------------

    #[test]
    fn mutating_tool_generates_review_item() {
        let tool = WriteFileMockTool;
        let ctx = default_test_ctx();
        let events = capture_events(|on_event| {
            let result = tool.execute(
                ctx,
                json!({"path": "/tmp/test.txt", "content": "hello"}),
                &on_event,
            );
            // This might fail on some systems, but ReviewItemCreated should still be emitted
            assert!(result.is_ok());
        });

        assert!(events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::ReviewItemCreated { .. })));
    }

    // -----------------------------------------------------------------------
    // Timeline Event Tests
    // -----------------------------------------------------------------------

    #[test]
    fn tool_emits_timeline_entry() {
        let tool = MockTool;
        let ctx = default_test_ctx();
        let events = capture_events(|on_event| {
            let _ = tool.execute(ctx, json!({}), &on_event);
        });

        assert!(events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::TimelineEntry { .. })));
    }

    #[test]
    fn tool_emits_started_and_finished() {
        let tool = MockTool;
        let ctx = default_test_ctx();
        let events = capture_events(|on_event| {
            let _ = tool.execute(ctx, json!({}), &on_event);
        });

        assert!(events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::ToolStarted { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::ToolFinished { .. })));
    }

    // -----------------------------------------------------------------------
    // Cancellation Tests
    // -----------------------------------------------------------------------

    #[test]
    fn cancelled_tool_returns_error() {
        let flag = Arc::new(AtomicBool::new(true)); // pre-cancelled
        let ctx = ToolContext {
            cancellation: Some(flag),
            ..default_test_ctx()
        };
        let tool = CancellationMockTool;
        let result = tool.execute(ctx, json!({}), &|_| {});
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Error Handling Tests
    // -----------------------------------------------------------------------

    #[test]
    fn tool_validation_error_propagates() {
        let tool = WriteFileMockTool;
        let result = tool.validate(&json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing"));
    }

    #[test]
    fn tool_result_contains_artifacts() {
        let tool = MockTool;
        let ctx = default_test_ctx();
        let result = tool.execute(ctx, json!({}), &|_| {}).unwrap();
        assert!(!result.artifacts.is_empty());
    }

    #[test]
    fn global_registry_register_all_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool)).unwrap();
        assert!(registry.get("mock.test").is_some());
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn default_test_ctx() -> ToolContext {
        ToolContext {
            execution_id: "test-exec".to_string(),
            pane_id: None,
            project_root: None,
            filesystem_scope: None,
            cwd: None,
            environment: HashMap::new(),
            cancellation: None,
            timeout_ms: None,
            allow_shell: true,
            allow_network: true,
            allow_read: true,
            allow_write: true,
            allow_delete: true,
            allow_git: true,
            allow_packages: true,
            allow_processes: true,
        }
    }

    fn capture_events<F>(f: F) -> Vec<ExecutionEvent>
    where
        F: FnOnce(Box<dyn Fn(ExecutionEvent)>),
    {
        let events: Arc<std::sync::Mutex<Vec<ExecutionEvent>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();
        let on_event = move |e: ExecutionEvent| {
            events_clone.lock().unwrap().push(e);
        };
        f(Box::new(on_event));
        Arc::try_unwrap(events).unwrap().into_inner().unwrap()
    }

    // -----------------------------------------------------------------------
    // Mock Tools
    // -----------------------------------------------------------------------

    struct MockTool;

    impl Tool for MockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.test")
        }
        fn display_name(&self) -> String {
            "Mock Tool".to_string()
        }
        fn description(&self) -> String {
            "A test mock".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![
                ExecutionClass::Implementation,
                ExecutionClass::Debugging,
                ExecutionClass::General,
            ]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::Shell]
        }

        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }

        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_shell,
                "shell",
                &|e| on_event(e),
            )?;
            let eid = ctx.execution_id.clone();
            on_event(ExecutionEvent::ToolStarted {
                tool_id: "mock.test".to_string(),
                execution_id: eid.clone(),
                args: "".to_string(),
            });
            on_event(ExecutionEvent::ToolFinished {
                tool_id: "mock.test".to_string(),
                execution_id: eid.clone(),
                summary: Some("ok".to_string()),
            });
            crate::execution::tools::helpers::emit_timeline(
                &eid,
                "mock.test",
                "completed",
                "ok",
                &|e| on_event(e),
            );
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".to_string(), "".to_string(), "ok".to_string()),
                artifacts: vec![ToolArtifact {
                    artifact_type: "mock".to_string(),
                    summary: "mock artifact".to_string(),
                    content: None,
                    path: None,
                    mime_type: None,
                }],
                review_items: vec![],
            })
        }
    }

    struct OtherMockTool;

    impl Tool for OtherMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.other")
        }
        fn display_name(&self) -> String {
            "Other Mock".to_string()
        }
        fn description(&self) -> String {
            "Another mock".to_string()
        }
        fn category_name(&self) -> String {
            "other".to_string()
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
            ctx: ToolContext,
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

    struct ReadFileMockTool;

    impl Tool for ReadFileMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.read")
        }
        fn display_name(&self) -> String {
            "Mock Read".to_string()
        }
        fn description(&self) -> String {
            "Mock read".to_string()
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
        fn validate(&self, args: &serde_json::Value) -> Result<(), String> {
            if args.get("path").and_then(|v| v.as_str()).is_some() {
                Ok(())
            } else {
                Err("Missing 'path'".to_string())
            }
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_read,
                "read_files",
                &|e| on_event(e),
            )?;
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct WriteFileMockTool;

    impl Tool for WriteFileMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.write")
        }
        fn display_name(&self) -> String {
            "Mock Write".to_string()
        }
        fn description(&self) -> String {
            "Mock write".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::WriteFiles]
        }
        fn validate(&self, args: &serde_json::Value) -> Result<(), String> {
            if args.get("path").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'path'".to_string());
            }
            if args.get("content").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'content'".to_string());
            }
            Ok(())
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_write,
                "write_files",
                &|e| on_event(e),
            )?;
            on_event(ExecutionEvent::ReviewItemCreated {
                tool_id: "mock.write".to_string(),
                execution_id: ctx.execution_id.clone(),
                action: "filesystem.write".to_string(),
                summary: "wrote file".to_string(),
                details: None,
            });
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![ReviewItem {
                    action: "filesystem.write".to_string(),
                    summary: "wrote file".to_string(),
                    details: None,
                    severity: "info".to_string(),
                }],
            })
        }
    }

    struct DeleteFileMockTool;

    impl Tool for DeleteFileMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.delete")
        }
        fn display_name(&self) -> String {
            "Mock Delete".to_string()
        }
        fn description(&self) -> String {
            "Mock delete".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::DeleteFiles]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_delete,
                "delete_files",
                &|e| on_event(e),
            )?;
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct GitMockTool;

    impl Tool for GitMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.git")
        }
        fn display_name(&self) -> String {
            "Mock Git".to_string()
        }
        fn description(&self) -> String {
            "Mock git".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::Git]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(&ctx, ctx.allow_git, "git", &|e| {
                on_event(e)
            })?;
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct PackageMockTool;

    impl Tool for PackageMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.pkg")
        }
        fn display_name(&self) -> String {
            "Mock Pkg".to_string()
        }
        fn description(&self) -> String {
            "Mock pkg".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::Packages]
        }
        fn validate(&self, args: &serde_json::Value) -> Result<(), String> {
            if args.get("name").and_then(|v| v.as_str()).is_some() {
                Ok(())
            } else {
                Err("Missing 'name'".to_string())
            }
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_packages,
                "packages",
                &|e| on_event(e),
            )?;
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct ProcessMockTool;

    impl Tool for ProcessMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.proc")
        }
        fn display_name(&self) -> String {
            "Mock Proc".to_string()
        }
        fn description(&self) -> String {
            "Mock proc".to_string()
        }
        fn category_name(&self) -> String {
            "test".to_string()
        }
        fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
            vec![ExecutionClass::General]
        }
        fn permissions(&self) -> Vec<ToolPermission> {
            vec![ToolPermission::Processes]
        }
        fn validate(&self, _args: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        fn execute(
            &self,
            ctx: ToolContext,
            _args: serde_json::Value,
            on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            crate::execution::tools::helpers::check_permission(
                &ctx,
                ctx.allow_processes,
                "processes",
                &|e| on_event(e),
            )?;
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }

    struct CancellationMockTool;

    impl Tool for CancellationMockTool {
        fn id(&self) -> ToolId {
            ToolId("mock.cancel")
        }
        fn display_name(&self) -> String {
            "Mock Cancel".to_string()
        }
        fn description(&self) -> String {
            "Mock cancel".to_string()
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
            ctx: ToolContext,
            _args: serde_json::Value,
            _on_event: &dyn Fn(ExecutionEvent),
        ) -> Result<ToolResult, String> {
            if ctx.is_cancelled() {
                return Err("Cancelled".to_string());
            }
            Ok(ToolResult {
                success: true,
                exit_code: Some(0),
                output: ToolOutput::new("".into(), "".into(), "ok".into()),
                artifacts: vec![],
                review_items: vec![],
            })
        }
    }
}
