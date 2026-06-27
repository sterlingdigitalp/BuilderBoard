# Filesystem Cost Report

## Operation Counts (Per Builder Request)

Based on a simulated Builder request executing typical tasks (`find_files`, `search_files`, and `read_file`), the filesystem operations counted are as follows:

- **Number of reads:** 5
- **Directory scans:** 10
- **Canonicalize calls:** 19
- **Metadata lookups:** 19
- **Repeated path resolution:** 3

## Unnecessary Filesystem Work

During operations like `find_files`, `search_files`, and `list_directory`, we iterate over entries produced by `fs::read_dir()`. The implementation performs unnecessary and expensive operations on every single entry returned from the directory iterator:

1.  **Unconditional `canonicalize()` calls:**
    The codebase calls `.canonicalize()` on every single file and directory encountered during traversal. This results in heavy I/O as the OS resolves symlinks and touches the disk for every child entry. In most scenarios, symlinks are rare, and this is completely unnecessary. `canonicalize` should only be called if an entry is known to be a symlink or if explicitly required for security verification, rather than eagerly for every directory member.
    *   Found in: `src-tauri/src/filesystem_tools/service.rs` inside `list_directory`, `walk_search_directory`, and `walk_find_directory`.

2.  **Unconditional `.metadata()` calls:**
    The code calls `entry.metadata()` on every directory entry just to determine if it is a directory or a file (e.g., `metadata.is_dir()`, `metadata.is_file()`). This is an unnecessary `stat` syscall. `std::fs::DirEntry` provides `.file_type()`, which typically retrieves the file type information directly from the directory listing without an additional `stat` system call (especially on Unix-like and modern Windows OSes).
    *   Found in: `src-tauri/src/filesystem_tools/service.rs` inside `list_directory`, `walk_search_directory`, and `walk_find_directory`.

3.  **Late Ignore Checking:**
    The checks for skipped directories (like `node_modules`) happen *after* `.canonicalize()` and `.metadata()` are executed for the directory itself. This means we do expensive filesystem operations on a directory just before deciding to skip traversing it.
