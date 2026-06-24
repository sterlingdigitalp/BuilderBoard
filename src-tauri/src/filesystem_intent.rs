//! Intent-based filesystem tool routing (Phase 8G).

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ReviewIntent {
    ArchitectureReview,
    SecurityReview,
    TechnicalDebtReview,
    ProductionReadinessReview,
    CodeQualityReview,
    FilesystemDiscovery,
    ProjectOverview,
}

impl ReviewIntent {
    pub const fn telemetry_label(self) -> &'static str {
        match self {
            Self::ArchitectureReview => "architecture_review",
            Self::SecurityReview => "security_review",
            Self::TechnicalDebtReview => "technical_debt_review",
            Self::ProductionReadinessReview => "production_readiness_review",
            Self::CodeQualityReview => "code_quality_review",
            Self::FilesystemDiscovery => "filesystem_discovery",
            Self::ProjectOverview => "project_overview",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FilesystemToolCall {
    ListDirectory { path: String },
    ReadFile { path: String },
    FindFiles { path: String, pattern: String },
    SearchFiles { path: String, query: String },
}

impl FilesystemToolCall {
    pub fn trace_tool_name(&self) -> &'static str {
        match self {
            Self::ListDirectory { .. } => "list_directory",
            Self::ReadFile { .. } => "read_file",
            Self::FindFiles { .. } => "find_files",
            Self::SearchFiles { .. } => "search_files",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RoutedFilesystemTools {
    pub intents: Vec<ReviewIntent>,
    pub bundle_label: String,
    pub tools: Vec<FilesystemToolCall>,
}

pub fn route_filesystem_tools(prompt: &str) -> RoutedFilesystemTools {
    let lower = prompt.to_ascii_lowercase();
    let intents = detect_intents(&lower);

    if is_package_json_focused_request(&lower) {
        let tools = vec![FilesystemToolCall::ReadFile {
            path: "package.json".to_string(),
        }];
        let routed = RoutedFilesystemTools {
            intents: vec![ReviewIntent::FilesystemDiscovery],
            bundle_label: ReviewIntent::FilesystemDiscovery.telemetry_label().to_string(),
            tools,
        };
        trace_intent_routing(&routed);
        return routed;
    }

    let (bundle_label, tools) = build_bundle_for_intents(&intents, &lower);
    let routed = RoutedFilesystemTools {
        intents,
        bundle_label,
        tools,
    };
    trace_intent_routing(&routed);
    routed
}

fn detect_intents(lower: &str) -> Vec<ReviewIntent> {
    let mut intents = Vec::new();

    if matches_filesystem_discovery(lower) {
        intents.push(ReviewIntent::FilesystemDiscovery);
    }
    if matches_security_review(lower) {
        intents.push(ReviewIntent::SecurityReview);
    }
    if matches_technical_debt_review(lower) {
        intents.push(ReviewIntent::TechnicalDebtReview);
    }
    if matches_production_readiness_review(lower) {
        intents.push(ReviewIntent::ProductionReadinessReview);
    }
    if matches_code_quality_review(lower) {
        intents.push(ReviewIntent::CodeQualityReview);
    }
    if matches_architecture_review(lower) {
        intents.push(ReviewIntent::ArchitectureReview);
    }

    if intents.is_empty() {
        intents.push(ReviewIntent::ProjectOverview);
    }

    intents
}

fn is_package_json_focused_request(lower: &str) -> bool {
    lower.contains("package.json")
        && !lower.contains("architecture")
        && !lower.contains("security")
        && !lower.contains("production readiness")
        && !lower.contains("production ready")
        && !lower.contains("technical debt")
        && !lower.contains("tech debt")
        && !lower.contains("maintainability")
        && !lower.contains("scalability")
        && !lower.contains("oauth")
}

fn matches_filesystem_discovery(lower: &str) -> bool {
    lower.contains("oauth")
        || lower.contains("typescript")
        || lower.contains("*.ts")
        || lower.contains("ts files")
        || lower.contains("find ")
        || lower.contains("search ")
        || lower.contains("take a look")
        || lower.contains("look at")
        || lower.contains("package.json")
}

fn matches_security_review(lower: &str) -> bool {
    lower.contains("security")
        || lower.contains("vulnerab")
        || lower.contains("credential")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("hardening")
        || lower.contains("audit auth")
        || lower.contains("auth audit")
}

fn matches_technical_debt_review(lower: &str) -> bool {
    lower.contains("technical debt")
        || lower.contains("tech debt")
        || lower.contains("debt audit")
        || lower.contains("identify technical debt")
        || lower.contains("maintainability")
        || lower.contains("legacy code")
        || lower.contains("refactor")
}

fn matches_production_readiness_review(lower: &str) -> bool {
    lower.contains("production readiness")
        || lower.contains("production ready")
        || lower.contains("readiness review")
        || lower.contains("go-live")
        || lower.contains("go live")
        || lower.contains("ship to production")
        || lower.contains("production review")
}

fn matches_code_quality_review(lower: &str) -> bool {
    lower.contains("code quality")
        || lower.contains("scalability")
        || lower.contains("onboarding")
        || lower.contains("implementation risk")
        || lower.contains("review code")
        || lower.contains("code review")
}

fn matches_architecture_review(lower: &str) -> bool {
    lower.contains("architecture")
        || lower.contains("codebase")
        || lower.contains("project structure")
        || lower.contains("review the project")
        || lower.contains("review this project")
        || lower.contains("system design")
        || lower.contains("review ")
        || lower.contains(" audit ")
        || lower.starts_with("audit ")
}

fn build_bundle_for_intents(
    intents: &[ReviewIntent],
    lower: &str,
) -> (String, Vec<FilesystemToolCall>) {
    let mut tools = Vec::new();
    let uses_full_architecture = intents.iter().any(|intent| {
        matches!(
            intent,
            ReviewIntent::ArchitectureReview
                | ReviewIntent::SecurityReview
                | ReviewIntent::TechnicalDebtReview
                | ReviewIntent::ProductionReadinessReview
                | ReviewIntent::CodeQualityReview
        )
    });

    if uses_full_architecture {
        extend_architecture_bundle(&mut tools);
    } else if intents.contains(&ReviewIntent::ProjectOverview) {
        extend_project_overview_bundle(&mut tools);
    }

    for intent in intents {
        match intent {
            ReviewIntent::SecurityReview => extend_security_bundle(&mut tools),
            ReviewIntent::TechnicalDebtReview => extend_technical_debt_bundle(&mut tools),
            ReviewIntent::ProductionReadinessReview => extend_production_readiness_bundle(&mut tools),
            ReviewIntent::FilesystemDiscovery => extend_filesystem_discovery_bundle(&mut tools, lower),
            ReviewIntent::ArchitectureReview
            | ReviewIntent::CodeQualityReview
            | ReviewIntent::ProjectOverview => {}
        }
    }

    dedupe_tools(&mut tools);
    let bundle_label = intents
        .iter()
        .map(|intent| intent.telemetry_label())
        .collect::<Vec<_>>()
        .join("+");
    (bundle_label, tools)
}

fn extend_project_overview_bundle(tools: &mut Vec<FilesystemToolCall>) {
    push_list_directory(tools, ".");
    push_read_file(tools, "package.json");
    push_read_file(tools, "README.md");
}

fn extend_architecture_bundle(tools: &mut Vec<FilesystemToolCall>) {
    extend_project_overview_bundle(tools);
    push_list_directory(tools, "src");
}

fn extend_security_bundle(tools: &mut Vec<FilesystemToolCall>) {
    for query in [
        "auth", "oauth", "token", "credential", "secret", "password", "key",
    ] {
        push_search_files(tools, ".", query);
    }
}

fn extend_technical_debt_bundle(tools: &mut Vec<FilesystemToolCall>) {
    push_find_files(tools, ".", "*.ts");
    push_find_files(tools, ".", "*.tsx");
}

fn extend_production_readiness_bundle(tools: &mut Vec<FilesystemToolCall>) {
    for path in ["tsconfig.json", "vite.config.ts", "Cargo.toml"] {
        push_read_file(tools, path);
    }
    push_search_files(tools, ".", "error");
}

fn extend_filesystem_discovery_bundle(tools: &mut Vec<FilesystemToolCall>, lower: &str) {
    if lower.contains("oauth") {
        push_search_files(tools, ".", "OAuth");
    }
    if lower.contains("typescript") || lower.contains("*.ts") || lower.contains("ts files") {
        push_find_files(tools, ".", "*.ts");
    }
    if lower.contains("take a look") || lower.contains("look at") {
        push_list_directory(tools, ".");
    }
}

fn push_list_directory(tools: &mut Vec<FilesystemToolCall>, path: &str) {
    tools.push(FilesystemToolCall::ListDirectory {
        path: path.to_string(),
    });
}

fn push_read_file(tools: &mut Vec<FilesystemToolCall>, path: &str) {
    tools.push(FilesystemToolCall::ReadFile {
        path: path.to_string(),
    });
}

fn push_find_files(tools: &mut Vec<FilesystemToolCall>, path: &str, pattern: &str) {
    tools.push(FilesystemToolCall::FindFiles {
        path: path.to_string(),
        pattern: pattern.to_string(),
    });
}

fn push_search_files(tools: &mut Vec<FilesystemToolCall>, path: &str, query: &str) {
    tools.push(FilesystemToolCall::SearchFiles {
        path: path.to_string(),
        query: query.to_string(),
    });
}

fn dedupe_tools(tools: &mut Vec<FilesystemToolCall>) {
    let mut deduped = Vec::new();
    for tool in tools.drain(..) {
        if deduped.iter().any(|existing| existing == &tool) {
            continue;
        }
        deduped.push(tool);
    }
    *tools = deduped;
}

fn trace_intent_routing(routed: &RoutedFilesystemTools) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() != Ok("1") {
        return;
    }

    let intent_labels = routed
        .intents
        .iter()
        .map(|intent| intent.telemetry_label())
        .collect::<Vec<_>>()
        .join(",");
    println!("INTENT_DETECTED={intent_labels}");
    println!("INTENT_BUNDLE={}", routed.bundle_label);
    println!(
        "FILESYSTEM_ROUTER_MATCHED={}",
        if routed.tools.is_empty() { "false" } else { "true" }
    );
    let tool_names = routed
        .tools
        .iter()
        .map(|tool| tool.trace_tool_name())
        .collect::<Vec<_>>()
        .join(",");
    println!("TOOLS_SELECTED={tool_names}");
}

#[cfg(test)]
mod tests {
    use super::{route_filesystem_tools, ReviewIntent};

    fn intents_for(prompt: &str) -> Vec<ReviewIntent> {
        route_filesystem_tools(prompt).intents
    }

    fn has_tool(prompt: &str, tool: &str) -> bool {
        route_filesystem_tools(prompt)
            .tools
            .iter()
            .any(|call| call.trace_tool_name() == tool)
    }

    #[test]
    fn production_readiness_review_triggers_filesystem_tools() {
        let routed = route_filesystem_tools("Perform a production readiness review");
        assert!(routed.intents.contains(&ReviewIntent::ProductionReadinessReview));
        assert!(has_tool("Perform a production readiness review", "list_directory"));
        assert!(has_tool("Perform a production readiness review", "read_file"));
    }

    #[test]
    fn security_concerns_trigger_security_bundle() {
        let routed = route_filesystem_tools("Find security concerns");
        assert!(routed.intents.contains(&ReviewIntent::SecurityReview));
        assert!(has_tool("Find security concerns", "search_files"));
        assert!(has_tool("Find security concerns", "list_directory"));
    }

    #[test]
    fn technical_debt_triggers_find_files() {
        let routed = route_filesystem_tools("Identify technical debt");
        assert!(routed.intents.contains(&ReviewIntent::TechnicalDebtReview));
        assert!(has_tool("Identify technical debt", "find_files"));
    }

    #[test]
    fn fallback_project_overview_for_unrecognized_prompts() {
        let routed = route_filesystem_tools("What is the weather today?");
        assert!(routed.intents.contains(&ReviewIntent::ProjectOverview));
        assert!(!routed.tools.is_empty());
    }

    #[test]
    fn package_json_focus_stays_minimal() {
        let routed = route_filesystem_tools("Review package.json");
        assert_eq!(routed.tools.len(), 1);
        assert!(matches!(
            routed.tools.first(),
            Some(super::FilesystemToolCall::ReadFile { path }) if path == "package.json"
        ));
    }

    #[test]
    fn phase8g_all_validation_prompts_route_tools() {
        let prompts = [
            "Review architecture",
            "Review package.json",
            "Find OAuth code",
            "Find security concerns",
            "Perform a production readiness review",
            "Identify technical debt",
            "Review maintainability",
            "Review scalability",
            "Review implementation risks",
            "Review onboarding",
        ];

        for prompt in prompts {
            let routed = route_filesystem_tools(prompt);
            assert!(
                !routed.tools.is_empty(),
                "prompt '{prompt}' should always select filesystem tools"
            );
        }
    }

    #[test]
    fn phase8g_validation_matrix_intents() {
        let cases = [
            ("Review architecture", ReviewIntent::ArchitectureReview),
            ("Find OAuth code", ReviewIntent::FilesystemDiscovery),
            ("Find security concerns", ReviewIntent::SecurityReview),
            (
                "Perform a production readiness review",
                ReviewIntent::ProductionReadinessReview,
            ),
            ("Identify technical debt", ReviewIntent::TechnicalDebtReview),
            ("Review maintainability", ReviewIntent::TechnicalDebtReview),
            ("Review scalability", ReviewIntent::CodeQualityReview),
            ("Review implementation risks", ReviewIntent::CodeQualityReview),
            ("Review onboarding", ReviewIntent::CodeQualityReview),
        ];

        for (prompt, expected) in cases {
            assert!(
                intents_for(prompt).contains(&expected),
                "prompt '{prompt}' should detect {:?}",
                expected
            );
        }
    }
}