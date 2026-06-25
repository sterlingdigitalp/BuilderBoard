use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::filesystem_tools::scope::ApprovedScope;
use crate::projects::repository::ProjectRepository;
use crate::storage::error::StorageError;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct ScopeCacheKey {
    project_id: String,
    approved_root: String,
}

#[derive(Clone, Debug)]
pub struct CachedProjectScope {
    pub project_id: String,
    pub project_name: String,
    pub approved_root: String,
    pub scope: ApprovedScope,
}

pub struct ProjectScopeCache {
    entries: Mutex<HashMap<ScopeCacheKey, CachedProjectScope>>,
}

impl ProjectScopeCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve(
        &self,
        connection: &Connection,
        project_id: &str,
    ) -> Result<CachedProjectScope, StorageError> {
        let project = ProjectRepository::get_by_id(connection, project_id)?;
        let key = ScopeCacheKey {
            project_id: project_id.to_string(),
            approved_root: project.approved_root.clone(),
        };

        if let Some(cached) = self.entries.lock().map_err(lock_error)?.get(&key).cloned() {
            return Ok(cached);
        }

        let scope = ApprovedScope::new(project.approved_root.clone())
            .map_err(|error| StorageError::InvalidInput(error.to_string()))?;
        let cached = CachedProjectScope {
            project_id: project_id.to_string(),
            project_name: project.name,
            approved_root: project.approved_root,
            scope,
        };

        self.entries
            .lock()
            .map_err(lock_error)?
            .insert(key, cached.clone());

        Ok(cached)
    }

    pub fn invalidate_project(&self, project_id: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|key, _| key.project_id != project_id);
        }
    }

    pub fn invalidate_all(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

impl Default for ProjectScopeCache {
    fn default() -> Self {
        Self::new()
    }
}

fn lock_error<T: std::fmt::Display>(error: T) -> StorageError {
    StorageError::InvalidInput(format!("project scope cache lock poisoned: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::repository::ProjectRepository;
    use crate::storage::db::Database;
    use crate::storage::error::StorageResult;

    fn temp_database(name: &str) -> Database {
        let path = std::env::temp_dir().join("builderboard-tests").join(name);
        let _ = std::fs::remove_file(&path);
        Database::initialize_at(path).expect("initialize database")
    }

    #[test]
    fn scope_cache_reuses_entry_for_same_project() -> StorageResult<()> {
        let database = temp_database("scope-cache-hit.db");
        let cache = ProjectScopeCache::new();
        let root = std::env::temp_dir().join("builderboard-scope-cache-root");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("package.json"), r#"{"name":"cache"}"#)?;

        let project_id = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, root.to_str().unwrap(), false)
                .map(|project| project.id)
        })?;

        let first =
            database.with_connection(|connection| cache.resolve(connection, &project_id))?;
        let second =
            database.with_connection(|connection| cache.resolve(connection, &project_id))?;

        assert_eq!(first.project_id, second.project_id);
        assert_eq!(first.approved_root, second.approved_root);
        Ok(())
    }

    #[test]
    fn scope_cache_invalidates_project_entries() -> StorageResult<()> {
        let database = temp_database("scope-cache-invalidate.db");
        let cache = ProjectScopeCache::new();
        let root = std::env::temp_dir().join("builderboard-scope-cache-invalidate-root");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("package.json"), r#"{"name":"cache"}"#)?;

        let project_id = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, root.to_str().unwrap(), false)
                .map(|project| project.id)
        })?;

        let first =
            database.with_connection(|connection| cache.resolve(connection, &project_id))?;
        cache.invalidate_project(&project_id);
        let second =
            database.with_connection(|connection| cache.resolve(connection, &project_id))?;
        assert_eq!(first.approved_root, second.approved_root);
        Ok(())
    }
}
