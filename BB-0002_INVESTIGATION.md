# BB-0002 Investigation Report

## Origin of Validation Failures

Repository tool validation failures originate in the filesystem path resolution logic, specifically within `src-tauri/src/filesystem_tools/scope.rs`.

When tools attempt to resolve a path to ensure it falls within the approved filesystem scope (the project root), they call `ApprovedScope::resolve_path`. Inside this method, `std::path::Path::canonicalize()` is used. In Rust, `canonicalize()` queries the filesystem and fails if the file or directory does not exist. This causes the tool execution to return a `FilesystemError::NotFound` (or bubble up as a `String` error in the `execute` method), resulting in a failed tool invocation.

**Code References:**
- `src-tauri/src/filesystem_tools/scope.rs:62` - `let canonical = requested.canonicalize().map_err(|error| ...)`
- `src-tauri/src/filesystem_tools/scope.rs:94` - `let canonical = resolved.canonicalize().map_err(|error| ...)`

## Complete Execution Path

1. **Tool Invocation:** The planner instructs a tool to execute (e.g., `directory.create`, `filesystem.write`, `search.grep`). The `Tool::execute` method is called. (e.g., `src-tauri/src/execution/tools/search.rs:82` for `GrepTool`).
2. **Context Resolution:** The tool attempts to resolve the requested path against the `ToolContext` using a helper function like `resolve_search_root` (`src-tauri/src/execution/tools/search.rs:341`), `resolve_dir_path` (`src-tauri/src/execution/tools/directory.rs:281`), or similar helpers in `filesystem.rs`.
3. **Scope Enforcement:** If `ctx.filesystem_scope` is present, the helper calls `scope.resolve_path(&candidate_str)` to ensure the path is within the approved root (`src-tauri/src/execution/tools/search.rs:363`, `src-tauri/src/execution/tools/directory.rs:299`).
4. **Canonicalization Failure:** `ApprovedScope::resolve_path` parses the requested path components and ultimately calls `canonicalize()` (`src-tauri/src/filesystem_tools/scope.rs:62` or `94`).
5. **Error Bubbling:** If the path does not exist on disk, `canonicalize()` returns an `std::io::Error::NotFound`. `resolve_path` wraps this in a `FilesystemError::NotFound`, which bubbles back up to the tool, causing the tool to fail with an error string before the actual tool logic runs.

## Root Cause

The root cause is the reliance on `std::path::Path::canonicalize()` for *all* path resolution and scope checking.

Because `canonicalize()` requires the path to physically exist on the filesystem to resolve symlinks and normalize paths, it fundamentally breaks any tool operation that operates on non-existent paths (e.g., `directory.create`, `filesystem.write` to a new file). Additionally, searches (`search.grep`, `search.glob`) that guess paths or use broad relative scopes may hit this if the resolution targets a path that isn't cleanly rooted or doesn't exist yet, rejecting the search entirely rather than returning 0 results.

## Relationship to BB-0001

**BB-0002 is causing (or severely exacerbating) BB-0001.**

Ledger entry BB-0001 states: *"Repository-scale discovery missions exhaust the planner before producing a result."*
Ledger entry BB-0002 states: *"Repository tool validation failures cause planner exhaustion. The planner repeatedly retries after failed tool invocations, consuming the tool-call budget without making meaningful progress."*

Because repository exploration involves querying for paths that may or may not exist (to determine project structure), tools frequently return validation errors (the `NotFound` errors from `canonicalize()`). The LLM planner interprets this as a failure of its tool call structure or arguments and attempts to retry or re-explore blindly. This triggers exponential retry cascades, which directly exhaust the planner budget (BB-0001) before it can gather the information it actually needs.

## Smallest Architectural Fix

The smallest architectural fix is to replace the strict `canonicalize()` usage in `ApprovedScope::resolve_path` with a purely lexical path normalization algorithm for scope checking.

Instead of hitting the filesystem to normalize `.` and `..` via `canonicalize()`, `resolve_path` should:
1. Lexically normalize the requested path (resolving `.` and `..` mathematically without checking the disk).
2. Check if the resulting normalized path starts with `self.canonical_root`.
3. Return the normalized path.

*Alternative (if symlink resolution is strictly required):*
Introduce a distinction between reading and writing paths. Keep `resolve_existing_path` (using `canonicalize()`) for read operations, but create a `resolve_new_path` that uses `canonicalize()` on the *parent* directory (which must exist) and simply joins the file component, allowing new files/directories to pass the scope check.