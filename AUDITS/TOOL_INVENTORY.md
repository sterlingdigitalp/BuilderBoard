# Native Builder Tool Inventory

This document catalogs all native tools available in the BuilderBoard runtime.

| Tool ID | Purpose | Schema Validation | Olympic Coverage |
|---------|---------|-------------------|------------------|
| `diagnostics.health` | Check the health and availability of external tools and dependencies. | See `tool_input_schema("diagnostics.health")` | None specified |
| `diagnostics.env` | Show environment information (OS, Rust version, etc.). | See `tool_input_schema("diagnostics.env")` | None specified |
| `directory.list` | List entries in a directory. | See `tool_input_schema("directory.list")` | None specified |
| `directory.create` | Create a directory and all parent directories. | See `tool_input_schema("directory.create")` | None specified |
| `filesystem.read` | Read the contents of a file. | See `tool_input_schema("filesystem.read")` | OPS-BRZ-004 |
| `filesystem.write` | Write content to a file. Creates parent directories if needed. | See `tool_input_schema("filesystem.write")` | None specified |
| `filesystem.edit` | Perform an exact string replacement in a file. | See `tool_input_schema("filesystem.edit")` | None specified |
| `filesystem.delete` | Delete a file or empty directory. | See `tool_input_schema("filesystem.delete")` | None specified |
| `git.status` | Show the working tree status. | See `tool_input_schema("git.status")` | OPS-BRZ-006 |
| `git.diff` | Show changes in the working tree or between commits. | See `tool_input_schema("git.diff")` | None specified |
| `git.commit` | Stage and commit changes. | See `tool_input_schema("git.commit")` | None specified |
| `git.log` | Show commit history. | See `tool_input_schema("git.log")` | None specified |
| `package.install` | Install a package using the detected package manager. | See `tool_input_schema("package.install")` | None specified |
| `package.uninstall` | Uninstall a package using the detected package manager. | See `tool_input_schema("package.uninstall")` | None specified |
| `package.list` | List installed packages using the detected package manager. | See `tool_input_schema("package.list")` | None specified |
| `process.list` | List running processes. | See `tool_input_schema("process.list")` | None specified |
| `process.kill` | Terminate a running process by PID. | See `tool_input_schema("process.kill")` | None specified |
| `search.grep` | Search file contents using regex patterns. | See `tool_input_schema("search.grep")` | OPS-BRZ-007 |
| `search.glob` | Find files matching a glob pattern. | See `tool_input_schema("search.glob")` | None specified |
| `shell` | Execute shell commands with real-time streaming of stdout/stderr. | See `tool_input_schema("shell")` | OPS-BRZ-005 |

## Detailed Tool Information

### `diagnostics.health`

**File:** `src-tauri/src/execution/tools/diagnostics.rs`
**Purpose:** Check the health and availability of external tools and dependencies.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("diagnostics.health")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `diagnostics.env`

**File:** `src-tauri/src/execution/tools/diagnostics.rs`
**Purpose:** Show environment information (OS, Rust version, etc.).
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("diagnostics.env")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `directory.list`

**File:** `src-tauri/src/execution/tools/directory.rs`
**Purpose:** List entries in a directory.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("directory.list")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            return Err("Missing required argument: 'path'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `directory.create`

**File:** `src-tauri/src/execution/tools/directory.rs`
**Purpose:** Create a directory and all parent directories.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("directory.create")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            return Err("Missing required argument: 'path'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `filesystem.read`

**File:** `src-tauri/src/execution/tools/filesystem.rs`
**Purpose:** Read the contents of a file.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("filesystem.read")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            Err("Missing required argument: 'path'".to_string())
        } else {
            Ok(())
        }
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** OPS-BRZ-004 (Single Tool Execution — Read File)

### `filesystem.write`

**File:** `src-tauri/src/execution/tools/filesystem.rs`
**Purpose:** Write content to a file. Creates parent directories if needed.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("filesystem.write")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        if args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
            && args.get("content").and_then(|v| v.as_str()).is_some()
        {
            return Ok(());
        }
        Err("Missing required arguments: 'path' (string) and 'content' (string)".to_string())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `filesystem.edit`

**File:** `src-tauri/src/execution/tools/filesystem.rs`
**Purpose:** Perform an exact string replacement in a file.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("filesystem.edit")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let old = args.get("old_string").and_then(|v| v.as_str());
        let new = args.get("new_string").and_then(|v| v.as_str());
        if path.is_none() {
            return Err("Missing 'path'".to_string());
        }
        if old.is_none() {
            return Err("Missing 'old_string'".to_string());
        }
        if new.is_none() {
            return Err("Missing 'new_string'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `filesystem.delete`

**File:** `src-tauri/src/execution/tools/filesystem.rs`
**Purpose:** Delete a file or empty directory.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("filesystem.delete")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            Ok(())
        } else {
            Err("Missing required argument: 'path'".to_string())
        }
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `git.status`

**File:** `src-tauri/src/execution/tools/git.rs`
**Purpose:** Show the working tree status.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("git.status")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** OPS-BRZ-006 (Single Tool Execution — Git Status)

### `git.diff`

**File:** `src-tauri/src/execution/tools/git.rs`
**Purpose:** Show changes in the working tree or between commits.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("git.diff")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `git.commit`

**File:** `src-tauri/src/execution/tools/git.rs`
**Purpose:** Stage and commit changes.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("git.commit")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let msg = args.get("message").and_then(|v| v.as_str());
        if msg.is_none() || msg.unwrap().is_empty() {
            return Err("Missing required argument: 'message'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `git.log`

**File:** `src-tauri/src/execution/tools/git.rs`
**Purpose:** Show commit history.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("git.log")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `package.install`

**File:** `src-tauri/src/execution/tools/package.rs`
**Purpose:** Install a package using the detected package manager.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("package.install")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let name = args.get("name").and_then(|v| v.as_str());
        if name.is_none() || name.unwrap().is_empty() {
            return Err("Missing required argument: 'name'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `package.uninstall`

**File:** `src-tauri/src/execution/tools/package.rs`
**Purpose:** Uninstall a package using the detected package manager.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("package.uninstall")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let name = args.get("name").and_then(|v| v.as_str());
        if name.is_none() || name.unwrap().is_empty() {
            return Err("Missing required argument: 'name'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `package.list`

**File:** `src-tauri/src/execution/tools/package.rs`
**Purpose:** List installed packages using the detected package manager.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("package.list")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `process.list`

**File:** `src-tauri/src/execution/tools/process.rs`
**Purpose:** List running processes.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("process.list")`
**Validation:**
```rust
    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `process.kill`

**File:** `src-tauri/src/execution/tools/process.rs`
**Purpose:** Terminate a running process by PID.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("process.kill")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let pid = args.get("pid").and_then(|v| v.as_u64());
        if pid.is_none() {
            return Err("Missing required argument: 'pid'".to_string());
        }
        let signal = args.get("signal").and_then(|v| v.as_str());
        if let Some(sig) = signal {
            let valid = [
                "SIGTERM", "SIGKILL", "SIGINT", "SIGHUP", "SIGSTOP", "SIGCONT",
            ];
            if !valid.contains(&sig) {
                return Err(format!("Invalid signal '{}'. Valid: {:?}", sig, valid));
            }
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `search.grep`

**File:** `src-tauri/src/execution/tools/search.rs`
**Purpose:** Search file contents using regex patterns.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("search.grep")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        if pattern.is_none() || pattern.unwrap().is_empty() {
            return Err("Missing required argument: 'pattern'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** OPS-BRZ-007 (Single Tool Execution — Search)

### `search.glob`

**File:** `src-tauri/src/execution/tools/search.rs`
**Purpose:** Find files matching a glob pattern.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("search.glob")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        if pattern.is_none() || pattern.unwrap().is_empty() {
            return Err("Missing required argument: 'pattern'".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** None documented.

### `shell`

**File:** `src-tauri/src/execution/tools/shell.rs`
**Purpose:** Execute shell commands with real-time streaming of stdout/stderr.
**Schema:** Defined in `capability_resolver.rs`, `tool_input_schema("shell")`
**Validation:**
```rust
    fn validate(&self, args: &Value) -> Result<(), String> {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required argument: 'command'".to_string())?;
        if cmd.trim().is_empty() {
            return Err("'command' must not be empty".to_string());
        }
        Ok(())
    }

```
**Caller:** Invoked via `ToolRegistry` through engine's `execute()` loop.
**Olympic Coverage:** OPS-BRZ-005 (Single Tool Execution — Shell Command)

