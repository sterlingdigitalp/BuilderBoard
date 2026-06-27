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
        let absolute_path = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
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
                    }
                    Component::RootDir | Component::Prefix(_) => {
                        return Err(FilesystemError::PathEscape(requested_path.to_string()));
                    }
                }
            }
            resolved
        };

        let canonical = canonicalize_longest_prefix(&absolute_path).map_err(|error| {
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
        let path = self.resolve_path(requested_path)?;
        if !path.exists() {
            return Err(FilesystemError::NotFound(requested_path.to_string()));
        }
        Ok(path)
    }
}

fn canonicalize_longest_prefix(path: &Path) -> Result<PathBuf, String> {
    let mut current = path.to_path_buf();
    let mut missing_components = Vec::new();

    loop {
        if current.exists() {
            let mut canonical = current.canonicalize().map_err(|e| e.to_string())?;
            for comp in missing_components.into_iter().rev() {
                canonical.push(comp);
            }

            let mut final_path = PathBuf::new();
            for comp in canonical.components() {
                match comp {
                    Component::ParentDir => { final_path.pop(); }
                    Component::CurDir => {}
                    _ => { final_path.push(comp); }
                }
            }
            return Ok(final_path);
        }

        let file_name = current.file_name().map(|s| s.to_owned());

        if let Some(parent) = current.parent().map(|p| p.to_path_buf()) {
            if let Some(name) = file_name {
                 missing_components.push(name);
            } else if current.ends_with("..") {
                 missing_components.push("..".into());
            } else if current.ends_with(".") {
                 missing_components.push(".".into());
            }
            current = parent;
        } else {
            return Err("Root does not exist".to_string());
        }
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

    #[test]
    fn resolve_absolute_in_scope_create_path() {
        let root = temp_dir("resolve-absolute");

        // Create a symlink structure similar to macOS /var -> /private/var
        // We'll create `private/var` inside our temp root, and symlink `var` to it.
        let private_var = root.join("private").join("var");
        fs::create_dir_all(&private_var).expect("create private/var");
        let var_symlink = root.join("var");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&private_var, &var_symlink).expect("create symlink");
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_dir;
            symlink_dir(&private_var, &var_symlink).expect("create symlink");
        }

        // The approved root is our simulated /var (the symlink)
        let scope = ApprovedScope::new(&var_symlink).expect("scope");

        // We want to create a new file in /var/missing.txt
        let absolute_path_str = var_symlink.join("missing.txt").to_str().unwrap().to_string();

        // Ensure it correctly canonicalizes the existing prefix (/var -> /private/var)
        // and appends the non-existent part (`missing.txt`) without a scope escape error.
        let resolved = scope.resolve_path(&absolute_path_str).expect("resolve should succeed for in-scope new file");

        assert_eq!(resolved, private_var.canonicalize().unwrap().join("missing.txt"));
    }
}
