use std::path::{Component, Path, PathBuf};

use crate::filesystem_tools::error::{FilesystemError, FilesystemResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedScope {
    pub approved_root: PathBuf,
    pub(crate) canonical_root: PathBuf,
}

impl ApprovedScope {
    pub(crate) fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }
}

impl ApprovedScope {
    pub fn new(approved_root: impl AsRef<Path>) -> FilesystemResult<Self> {
        let approved_root = approved_root.as_ref();
        if !approved_root.exists() {
            return Err(FilesystemError::NotFound(
                approved_root.display().to_string(),
            ));
        }
        if !approved_root.is_dir() {
            return Err(FilesystemError::NotADirectory(
                approved_root.display().to_string(),
            ));
        }

        let canonical_root = approved_root.canonicalize().map_err(|error| {
            FilesystemError::Io(format!(
                "failed to canonicalize approved root {}: {error}",
                approved_root.display()
            ))
        })?;

        Ok(Self {
            approved_root: approved_root.to_path_buf(),
            canonical_root,
        })
    }

    pub fn root_display(&self) -> String {
        self.canonical_root.display().to_string()
    }

    pub fn resolve_path(&self, requested_path: &str) -> FilesystemResult<PathBuf> {
        let requested_path = requested_path.trim();
        if requested_path.contains('\0') {
            return Err(FilesystemError::InvalidInput(
                "path contains null byte".to_string(),
            ));
        }

        if requested_path.is_empty() || requested_path == "." {
            return Ok(self.canonical_root.clone());
        }

        let requested = Path::new(requested_path);
        if requested.is_absolute() {
            let canonical = requested.canonicalize().map_err(|error| {
                FilesystemError::NotFound(format!(
                    "failed to resolve path {}: {error}",
                    requested_path
                ))
            })?;
            if !is_within_root(&canonical, &self.canonical_root) {
                return Err(FilesystemError::PathEscape(requested_path.to_string()));
            }
            return Ok(canonical);
        }

        let mut resolved = self.canonical_root.clone();
        for component in requested.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => resolved.push(part),
                Component::ParentDir => {
                    if resolved == self.canonical_root {
                        return Err(FilesystemError::PathEscape(requested_path.to_string()));
                    }
                    resolved.pop();
                    if !is_within_root(&resolved, &self.canonical_root) {
                        return Err(FilesystemError::PathEscape(requested_path.to_string()));
                    }
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(FilesystemError::PathEscape(requested_path.to_string()));
                }
            }
        }

        let canonical = resolved.canonicalize().map_err(|error| {
            FilesystemError::NotFound(format!(
                "failed to resolve path {}: {error}",
                requested_path
            ))
        })?;

        if !is_within_root(&canonical, &self.canonical_root) {
            return Err(FilesystemError::PathEscape(requested_path.to_string()));
        }

        Ok(canonical)
    }

    pub fn resolve_existing_path(&self, requested_path: &str) -> FilesystemResult<PathBuf> {
        self.resolve_path(requested_path)
    }

    pub fn resolve_create_path(&self, requested_path: &str) -> FilesystemResult<PathBuf> {
        let requested_path = requested_path.trim();
        if requested_path.contains('\0') {
            return Err(FilesystemError::InvalidInput(
                "path contains null byte".to_string(),
            ));
        }

        if requested_path.is_empty() || requested_path == "." {
            return Ok(self.canonical_root.clone());
        }

        let requested = Path::new(requested_path);
        let candidate = if requested.is_absolute() {
            normalize_absolute(requested, requested_path)?
        } else {
            normalize_relative(&self.canonical_root, requested, requested_path)?
        };

        if candidate.exists() {
            let canonical = candidate.canonicalize().map_err(|error| {
                FilesystemError::NotFound(format!(
                    "failed to resolve path {}: {error}",
                    requested_path
                ))
            })?;
            if !is_within_root(&canonical, &self.canonical_root) {
                return Err(FilesystemError::PathEscape(requested_path.to_string()));
            }
            return Ok(canonical);
        }

        let mut existing_ancestor = candidate
            .parent()
            .ok_or_else(|| FilesystemError::PathEscape(requested_path.to_string()))?
            .to_path_buf();

        while !existing_ancestor.exists() {
            if existing_ancestor == self.canonical_root || !existing_ancestor.pop() {
                return Err(FilesystemError::NotFound(format!(
                    "failed to resolve parent for {}",
                    requested_path
                )));
            }
        }

        let canonical_ancestor = existing_ancestor.canonicalize().map_err(|error| {
            FilesystemError::NotFound(format!(
                "failed to resolve parent for {}: {error}",
                requested_path
            ))
        })?;
        if !is_within_root(&canonical_ancestor, &self.canonical_root) {
            return Err(FilesystemError::PathEscape(requested_path.to_string()));
        }

        Ok(candidate)
    }
}

pub fn is_within_root(candidate: &Path, canonical_root: &Path) -> bool {
    candidate.starts_with(canonical_root)
}

fn normalize_absolute(path: &Path, original: &str) -> FilesystemResult<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(FilesystemError::PathEscape(original.to_string()));
                }
            }
        }
    }
    Ok(normalized)
}

fn normalize_relative(root: &Path, path: &Path, original: &str) -> FilesystemResult<PathBuf> {
    let mut normalized = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if normalized == root {
                    return Err(FilesystemError::PathEscape(original.to_string()));
                }
                normalized.pop();
                if !is_within_root(&normalized, root) {
                    return Err(FilesystemError::PathEscape(original.to_string()));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(FilesystemError::PathEscape(original.to_string()));
            }
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("builderboard-fs-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn resolve_relative_path_within_root() {
        let root = temp_dir("resolve-relative");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(nested.join("file.txt"), "hello").expect("write file");

        let scope = ApprovedScope::new(&root).expect("scope");
        let resolved = scope.resolve_path("nested/file.txt").expect("resolve");
        assert_eq!(
            resolved,
            nested
                .join("file.txt")
                .canonicalize()
                .expect("canonicalize")
        );
    }

    #[test]
    fn reject_parent_traversal() {
        let root = temp_dir("reject-traversal");
        let scope = ApprovedScope::new(&root).expect("scope");
        let result = scope.resolve_path("../secret");
        assert!(matches!(result, Err(FilesystemError::PathEscape(_))));
    }

    #[test]
    fn reject_symlink_escape() {
        let root = temp_dir("reject-symlink");
        let outside = temp_dir("reject-symlink-outside");
        fs::write(outside.join("secret.txt"), "secret").expect("write outside");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&outside, root.join("escape")).expect("create symlink");
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_dir;
            symlink_dir(&outside, root.join("escape")).expect("create symlink");
        }

        let scope = ApprovedScope::new(&root).expect("scope");
        let result = scope.resolve_path("escape/secret.txt");
        assert!(matches!(result, Err(FilesystemError::PathEscape(_))));
    }

    #[test]
    fn resolve_create_path_allows_new_file_inside_root() {
        let root = temp_dir("resolve-create-file");
        let scope = ApprovedScope::new(&root).expect("scope");

        let resolved = scope
            .resolve_create_path("docs/test.md")
            .expect("resolve create path");

        assert_eq!(resolved, root.canonicalize().unwrap().join("docs/test.md"));
    }

    #[test]
    fn resolve_create_path_rejects_traversal() {
        let root = temp_dir("resolve-create-traversal");
        let scope = ApprovedScope::new(&root).expect("scope");

        let result = scope.resolve_create_path("../outside.md");

        assert!(matches!(result, Err(FilesystemError::PathEscape(_))));
    }

    #[test]
    fn resolve_create_path_rejects_symlink_parent_escape() {
        let root = temp_dir("resolve-create-symlink");
        let outside = temp_dir("resolve-create-symlink-outside");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&outside, root.join("escape")).expect("create symlink");
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_dir;
            symlink_dir(&outside, root.join("escape")).expect("create symlink");
        }

        let scope = ApprovedScope::new(&root).expect("scope");
        let result = scope.resolve_create_path("escape/new.md");

        assert!(matches!(result, Err(FilesystemError::PathEscape(_))));
    }
}
