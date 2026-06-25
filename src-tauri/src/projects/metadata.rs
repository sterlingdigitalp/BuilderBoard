use serde_json::{Map, Value};

pub const PROJECT_KIND_KEY: &str = "projectKind";
pub const PROJECT_NAME_KEY: &str = "projectName";
pub const PROJECT_CODE_KEY: &str = "projectCode";
pub const APPROVED_ROOT_KEY: &str = "filesystemApprovedRoot";

pub fn parse_metadata(metadata_json: Option<&str>) -> Map<String, Value> {
    metadata_json
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

pub fn project_metadata_map(
    project_name: &str,
    project_code: &str,
    approved_root: &str,
) -> Map<String, Value> {
    let mut metadata = Map::new();
    metadata.insert(
        PROJECT_KIND_KEY.to_string(),
        Value::String("folder".to_string()),
    );
    metadata.insert(
        PROJECT_NAME_KEY.to_string(),
        Value::String(project_name.to_string()),
    );
    metadata.insert(
        PROJECT_CODE_KEY.to_string(),
        Value::String(project_code.to_string()),
    );
    metadata.insert(
        APPROVED_ROOT_KEY.to_string(),
        Value::String(approved_root.to_string()),
    );
    metadata
}

pub fn project_code_base(name: &str) -> String {
    let chars: Vec<char> = name
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect();
    if chars.is_empty() {
        return "Pr".to_string();
    }
    if chars.len() == 1 {
        return format!(
            "{}{}",
            chars[0].to_ascii_uppercase(),
            chars[0].to_ascii_lowercase()
        );
    }

    format!(
        "{}{}",
        chars[0].to_ascii_uppercase(),
        chars[1].to_ascii_lowercase()
    )
}

pub fn allocate_project_code(name: &str, existing_codes: &[String]) -> String {
    let base = project_code_base(name);
    if !existing_codes.iter().any(|code| code == &base) {
        return base;
    }

    let mut suffix = 2u32;
    loop {
        let candidate = format!("{base}{suffix}");
        if !existing_codes.iter().any(|code| code == &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_code_base_examples() {
        assert_eq!(project_code_base("PepFox"), "Pe");
        assert_eq!(project_code_base("Assymetry"), "As");
        assert_eq!(project_code_base("AgentHive"), "Ag");
        assert_eq!(project_code_base("Arete"), "Ar");
        assert_eq!(project_code_base("Longevity"), "Lo");
        assert_eq!(project_code_base("PeopleOps"), "Pe");
    }

    #[test]
    fn allocate_project_code_handles_collision() {
        let existing = vec!["Pe".to_string()];
        assert_eq!(allocate_project_code("PeopleOps", &existing), "Pe2");
        let existing = vec!["Pe".to_string(), "Pe2".to_string()];
        assert_eq!(allocate_project_code("People", &existing), "Pe3");
    }
}
