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
}

pub fn is_within_root(candidate: &Path, canonical_root: &Path) -> bool {
    candidate.starts_with(canonical_root)
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
}
