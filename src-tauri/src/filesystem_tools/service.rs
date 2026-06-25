use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::filesystem_tools::error::{FilesystemError, FilesystemResult};
use crate::filesystem_tools::models::{
    DirectoryEntryDto, FindFilesResult, ListDirectoryResult, ReadFileResult, SearchFilesResult,
    SearchMatchFileDto, SearchMatchLineDto,
};
use crate::filesystem_tools::scan_context::ScanContext;
use crate::filesystem_tools::scope::{is_within_root, ApprovedScope};

pub const MAX_READ_FILE_BYTES: u64 = 1_048_576;
pub const MAX_SEARCH_FILE_BYTES: u64 = 5_242_880;
pub const MAX_SEARCH_MATCHES: usize = 100;
pub const MAX_FIND_RESULTS: usize = 150;
pub const MAX_LIST_DIRECTORY_ENTRIES: usize = 100;
pub const MAX_PROMPT_INJECTION_BYTES: usize = 24_576;
pub const MAX_INJECTED_READ_FILE_CHARS: usize = 8_000;
pub const MAX_INJECTED_FIND_PATHS: usize = 80;
pub const MAX_INJECTED_SEARCH_FILES: usize = 20;
pub const BINARY_SAMPLE_BYTES: usize = 8_192;

pub struct FilesystemService;

impl FilesystemService {
    pub fn list_directory(
        scope: &ApprovedScope,
        path: &str,
    ) -> FilesystemResult<ListDirectoryResult> {
        let resolved = scope.resolve_existing_path(path)?;
        if !resolved.is_dir() {
            return Err(FilesystemError::NotADirectory(path.to_string()));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&resolved).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let metadata = entry.metadata().map_err(io_error)?;
            let entry_path = entry.path();
            let canonical_entry = entry_path.canonicalize().map_err(io_error)?;
            if !is_within_root(&canonical_entry, scope.canonical_root()) {
                continue;
            }

            let entry_type = if metadata.is_dir() {
                "directory"
            } else if metadata.is_file() {
                "file"
            } else if metadata.is_symlink() {
                "symlink"
            } else {
                "other"
            };

            entries.push(DirectoryEntryDto {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: display_relative(scope.canonical_root(), &canonical_entry),
                entry_type: entry_type.to_string(),
                size_bytes: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified_at: modified_timestamp(&metadata),
            });

            if entries.len() >= MAX_LIST_DIRECTORY_ENTRIES {
                break;
            }
        }

        entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

        Ok(ListDirectoryResult {
            path: display_relative(scope.canonical_root(), &resolved),
            entries,
        })
    }

    pub fn read_file(scope: &ApprovedScope, path: &str) -> FilesystemResult<ReadFileResult> {
        let resolved = scope.resolve_existing_path(path)?;
        if !resolved.is_file() {
            return Err(FilesystemError::NotAFile(path.to_string()));
        }

        let metadata = fs::metadata(&resolved).map_err(io_error)?;
        let size_bytes = metadata.len();
        if size_bytes > MAX_READ_FILE_BYTES {
            return Err(FilesystemError::FileTooLarge {
                path: path.to_string(),
                size_bytes,
                limit_bytes: MAX_READ_FILE_BYTES,
            });
        }

        let bytes = fs::read(&resolved).map_err(io_error)?;
        if is_probably_binary(&bytes) {
            return Err(FilesystemError::BinaryFile(path.to_string()));
        }

        let content =
            String::from_utf8(bytes).map_err(|_| FilesystemError::BinaryFile(path.to_string()))?;

        Ok(ReadFileResult {
            path: display_relative(scope.canonical_root(), &resolved),
            content,
            size_bytes,
            truncated: false,
        })
    }

    pub fn search_files(
        scope: &ApprovedScope,
        path: &str,
        query: &str,
    ) -> FilesystemResult<SearchFilesResult> {
        Self::search_files_with_context(scope, path, query, &mut ScanContext::direct_command(path))
    }

    pub fn search_files_with_context(
        scope: &ApprovedScope,
        path: &str,
        query: &str,
        context: &mut ScanContext,
    ) -> FilesystemResult<SearchFilesResult> {
        let query = query.trim();
        if query.is_empty() {
            return Err(FilesystemError::InvalidInput(
                "search query cannot be empty".to_string(),
            ));
        }

        let resolved = scope.resolve_existing_path(path)?;
        if !resolved.is_dir() {
            return Err(FilesystemError::NotADirectory(path.to_string()));
        }

        let mut matches = Vec::new();
        let mut total_matches = 0usize;
        walk_search_directory(
            scope,
            &resolved,
            query,
            &mut matches,
            &mut total_matches,
            context,
        )?;

        Ok(SearchFilesResult {
            path: display_relative(scope.canonical_root(), &resolved),
            query: query.to_string(),
            matches,
        })
    }

    pub fn find_files(
        scope: &ApprovedScope,
        path: &str,
        pattern: &str,
    ) -> FilesystemResult<FindFilesResult> {
        Self::find_files_with_context(scope, path, pattern, &mut ScanContext::direct_command(path))
    }

    pub fn find_files_with_context(
        scope: &ApprovedScope,
        path: &str,
        pattern: &str,
        context: &mut ScanContext,
    ) -> FilesystemResult<FindFilesResult> {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return Err(FilesystemError::InvalidInput(
                "find pattern cannot be empty".to_string(),
            ));
        }

        let resolved = scope.resolve_existing_path(path)?;
        if !resolved.is_dir() {
            return Err(FilesystemError::NotADirectory(path.to_string()));
        }

        let mut matches = Vec::new();
        walk_find_directory(scope, &resolved, pattern, &mut matches, context)?;

        matches.sort();
        Ok(FindFilesResult {
            path: display_relative(scope.canonical_root(), &resolved),
            pattern: pattern.to_string(),
            matches,
        })
    }
}

fn walk_search_directory(
    scope: &ApprovedScope,
    directory: &Path,
    query: &str,
    matches: &mut Vec<SearchMatchFileDto>,
    total_matches: &mut usize,
    context: &mut ScanContext,
) -> FilesystemResult<()> {
    if context.scan_limit_reached || *total_matches >= MAX_SEARCH_MATCHES {
        return Ok(());
    }

    for entry in fs::read_dir(directory).map_err(io_error)? {
        if context.scan_limit_reached || *total_matches >= MAX_SEARCH_MATCHES {
            break;
        }

        if !context.record_scan() {
            break;
        }

        let entry = entry.map_err(io_error)?;
        let entry_path = entry.path();
        let canonical_entry = entry_path.canonicalize().map_err(io_error)?;
        if !is_within_root(&canonical_entry, scope.canonical_root()) {
            continue;
        }

        let metadata = entry.metadata().map_err(io_error)?;
        if metadata.is_dir() {
            let dir_name = entry.file_name().to_string_lossy().into_owned();
            if context.should_skip_directory(&dir_name) {
                continue;
            }
            walk_search_directory(
                scope,
                &canonical_entry,
                query,
                matches,
                total_matches,
                context,
            )?;
            continue;
        }

        if !metadata.is_file() || metadata.len() > MAX_SEARCH_FILE_BYTES {
            continue;
        }

        if let Some(mut file_matches) = search_text_file(&canonical_entry, query, total_matches)? {
            file_matches.path = display_relative(scope.canonical_root(), &canonical_entry);
            matches.push(file_matches);
        }
    }

    Ok(())
}

fn search_text_file(
    path: &Path,
    query: &str,
    total_matches: &mut usize,
) -> FilesystemResult<Option<SearchMatchFileDto>> {
    let file = fs::File::open(path).map_err(io_error)?;
    let mut reader = BufReader::new(file);
    let mut sample = Vec::with_capacity(BINARY_SAMPLE_BYTES);
    let sample_len = reader
        .fill_buf()
        .map_err(io_error)?
        .iter()
        .take(BINARY_SAMPLE_BYTES)
        .copied()
        .inspect(|byte| sample.push(*byte))
        .count();
    if is_probably_binary(&sample[..sample_len]) {
        return Ok(None);
    }

    let mut line_matches = Vec::new();
    let mut buffer = String::new();
    let mut line_number = 0u32;

    loop {
        buffer.clear();
        let read = match reader.read_line(&mut buffer) {
            Ok(read) => read,
            Err(error)
                if error.kind() == std::io::ErrorKind::InvalidData
                    || error.to_string().contains("UTF-8") =>
            {
                return Ok(None);
            }
            Err(error) => return Err(io_error(error)),
        };
        if read == 0 {
            break;
        }
        line_number += 1;
        if buffer
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
        {
            line_matches.push(SearchMatchLineDto {
                line_number,
                line: buffer.trim_end().to_string(),
            });
            *total_matches += 1;
            if *total_matches >= MAX_SEARCH_MATCHES {
                break;
            }
        }
    }

    if line_matches.is_empty() {
        return Ok(None);
    }

    Ok(Some(SearchMatchFileDto {
        path: path.display().to_string(),
        matches: line_matches,
    }))
}

fn walk_find_directory(
    scope: &ApprovedScope,
    directory: &Path,
    pattern: &str,
    matches: &mut Vec<String>,
    context: &mut ScanContext,
) -> FilesystemResult<()> {
    if context.scan_limit_reached || matches.len() >= MAX_FIND_RESULTS {
        return Ok(());
    }

    for entry in fs::read_dir(directory).map_err(io_error)? {
        if context.scan_limit_reached || matches.len() >= MAX_FIND_RESULTS {
            break;
        }

        if !context.record_scan() {
            break;
        }

        let entry = entry.map_err(io_error)?;
        let entry_path = entry.path();
        let canonical_entry = entry_path.canonicalize().map_err(io_error)?;
        if !is_within_root(&canonical_entry, scope.canonical_root()) {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata().map_err(io_error)?;

        if metadata.is_dir() {
            if context.should_skip_directory(&file_name) {
                continue;
            }
            walk_find_directory(scope, &canonical_entry, pattern, matches, context)?;
            continue;
        }

        if metadata.is_file() && glob_matches(pattern, &file_name) {
            matches.push(display_relative(scope.canonical_root(), &canonical_entry));
            if matches.len() >= MAX_FIND_RESULTS {
                return Ok(());
            }
        }
    }

    Ok(())
}

pub fn glob_matches(pattern: &str, candidate: &str) -> bool {
    glob_match_recursive(pattern.as_bytes(), candidate.as_bytes(), 0, 0)
}

fn glob_match_recursive(pattern: &[u8], candidate: &[u8], p: usize, c: usize) -> bool {
    if p == pattern.len() {
        return c == candidate.len();
    }

    if pattern[p] == b'*' {
        if p + 1 == pattern.len() {
            return true;
        }
        for index in c..=candidate.len() {
            if glob_match_recursive(pattern, candidate, p + 1, index) {
                return true;
            }
        }
        return false;
    }

    if c == candidate.len() {
        return false;
    }

    if pattern[p] == b'?' || pattern[p] == candidate[c] {
        return glob_match_recursive(pattern, candidate, p + 1, c + 1);
    }

    false
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                ".".to_string()
            } else {
                relative.display().to_string()
            }
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn modified_timestamp(metadata: &fs::Metadata) -> Option<String> {
    metadata.modified().ok().and_then(system_time_to_rfc3339)
}

fn system_time_to_rfc3339(value: SystemTime) -> Option<String> {
    let datetime: DateTime<Utc> = value.into();
    Some(datetime.to_rfc3339())
}

fn is_probably_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

fn io_error(error: std::io::Error) -> FilesystemError {
    FilesystemError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_examples() {
        assert!(glob_matches("*.ts", "index.ts"));
        assert!(glob_matches("*.tsx", "Button.tsx"));
        assert!(glob_matches("package.json", "package.json"));
        assert!(glob_matches("README.md", "README.md"));
        assert!(!glob_matches("*.ts", "index.tsx"));
    }
}
