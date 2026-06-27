# Filesystem Cost Report

## Executive Summary

This investigation measured the filesystem cost of a single Builder request consisting of typical `find_files`, `search_files`, and `read_file` operations. The trace revealed a disproportionate amount of metadata and canonicalization overhead compared to actual reads (19 `canonicalize` and 19 `metadata` calls for only 5 file reads and 10 directory scans). The highest-impact inefficiencies discovered are:
1. **Unconditional Canonicalization**: Eagerly resolving every directory member using `.canonicalize()` before checking if the path is needed or if it is an actual symlink.
2. **Redundant Metadata Syscalls**: Making an explicit `.metadata()` call on every directory entry instead of retrieving the basic file type from `DirEntry::file_type()`.
3. **Late Ignore List Enforcement**: Calling `canonicalize()` and `metadata()` on directories (such as `node_modules`) *before* checking whether the directory is in the ignore list.

## Operation Counts (Per Builder Request)

Based on a simulated Builder request executing typical tasks (`find_files`, `search_files`, and `read_file`), the filesystem operations counted are as follows:

- **Number of reads:** 5
- **Directory scans:** 10
- **Canonicalize calls:** 19
- **Metadata lookups:** 19
- **Repeated path resolution:** 3

## Unnecessary Filesystem Work and Recommendations

During operations like `find_files`, `search_files`, and `list_directory`, we iterate over entries produced by `fs::read_dir()`. The implementation performs unnecessary and expensive operations on every single entry returned from the directory iterator:

### 1. Unconditional `canonicalize()` calls
The codebase calls `.canonicalize()` on every single file and directory encountered during traversal. This results in heavy I/O as the OS resolves symlinks and touches the disk for every child entry. In most scenarios, symlinks are rare, and this is completely unnecessary. `canonicalize` should only be called if an entry is known to be a symlink or if explicitly required for security verification, rather than eagerly for every directory member.
*   **Found in:** `src-tauri/src/filesystem_tools/service.rs` inside `list_directory`, `walk_search_directory`, and `walk_find_directory`.
*   **Recommendation Status:** Candidate for Builder C Review

### 2. Unconditional `.metadata()` calls
The code calls `entry.metadata()` on every directory entry just to determine if it is a directory or a file (e.g., `metadata.is_dir()`, `metadata.is_file()`). This is an unnecessary `stat` syscall. `std::fs::DirEntry` provides `.file_type()`, which typically retrieves the file type information directly from the directory listing without an additional `stat` system call (especially on Unix-like and modern Windows OSes).
*   **Found in:** `src-tauri/src/filesystem_tools/service.rs` inside `list_directory`, `walk_search_directory`, and `walk_find_directory`.
*   **Recommendation Status:** Candidate for Builder C Review

### 3. Late Ignore Checking
The checks for skipped directories (like `node_modules`) happen *after* `.canonicalize()` and `.metadata()` are executed for the directory itself. This means we do expensive filesystem operations on a directory just before deciding to skip traversing it.
*   **Found in:** `src-tauri/src/filesystem_tools/service.rs` inside `walk_search_directory` and `walk_find_directory`.
*   **Recommendation Status:** Candidate for Builder C Review
