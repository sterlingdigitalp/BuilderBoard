pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".next",
    "dist",
    "build",
    "coverage",
    ".cache",
    "tmp",
    "temp",
];

pub fn is_default_ignored_dir(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    DEFAULT_IGNORED_DIRS
        .iter()
        .any(|ignored| normalized == *ignored)
}

pub fn path_targets_ignored_segment(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    DEFAULT_IGNORED_DIRS.iter().any(|ignored| {
        normalized == *ignored
            || normalized.starts_with(&format!("{ignored}/"))
            || normalized.contains(&format!("/{ignored}/"))
            || normalized.ends_with(&format!("/{ignored}"))
    })
}

pub fn prompt_explicitly_allows_ignored(prompt: &str, scan_path: &str) -> bool {
    let prompt_lower = prompt.to_ascii_lowercase();
    if path_targets_ignored_segment(scan_path) {
        return true;
    }

    DEFAULT_IGNORED_DIRS
        .iter()
        .any(|ignored| prompt_lower.contains(ignored))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_dir_names_match_defaults() {
        assert!(is_default_ignored_dir("node_modules"));
        assert!(is_default_ignored_dir(".git"));
        assert!(!is_default_ignored_dir("src"));
    }

    #[test]
    fn path_targets_ignored_segment_detects_nested_paths() {
        assert!(path_targets_ignored_segment("node_modules/lodash"));
        assert!(path_targets_ignored_segment("vendor/node_modules/pkg"));
        assert!(!path_targets_ignored_segment("src/components"));
    }
}