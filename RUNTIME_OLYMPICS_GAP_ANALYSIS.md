# Runtime Olympics Gap Analysis

This document compares the `BuilderBoard 1.0-Core Definition.md` against the existing Phase 0 Runtime Olympics (`docs/runtime/PHASE0_OLYMPICS.md`) to identify Core Promise requirements that currently have no corresponding Olympic event.

## 1. Modifying Files, Fixing Bugs, and Implementing Changes
**Core Definition Requirement:**
> "This includes: [...] modifying files [...] fixing bugs [...] implementing requested changes"
> "Builders cannot reliably modify projects." (Version 1 Failure)

**Olympics Gap:**
Current Bronze events only cover read-only tool operations: `OPS-BRZ-004` (Read File), `OPS-BRZ-005` (Shell Command), `OPS-BRZ-006` (Git Status), and `OPS-BRZ-007` (Search). There are no Olympic events that verify a Builder can successfully modify a file, fix a bug, or implement requested changes.

## 2. Running Builds and Tests
**Core Definition Requirement:**
> "This includes: [...] running builds [...] running tests"

**Olympics Gap:**
There are no Bronze, Silver, or Gold events that explicitly verify a Builder executing a build command, running a test suite, or iterating based on test failures. While `OPS-BRZ-005` (Shell Command) allows running shell commands, there is no specific event validating the capability to execute and interpret standard build/test workflows.

## 3. Selecting Different Models
**Core Definition Requirement:**
> "Each Builder can use a different language model."
> "Version 1 Success: [...] select Builder models"

**Olympics Gap:**
No Olympic event verifies the ability to select different language models per Builder pane or guarantees that they operate using distinct, user-selected models.

## 4. Assigning Different Projects / Repositories
**Core Definition Requirement:**
> "Each Builder can work on a different software project."
> "Each Builder has its own: repository..."
> "Version 1 Success: [...] assign four different software projects"

**Olympics Gap:**
While `OPS-GLD-001` and `OPS-GLD-002` verify multi-pane independent operations, they do not explicitly require or test assigning four *different* repositories to the four panes simultaneously to ensure complete context isolation across different projects.

## 5. Continuing Conversations (Multi-turn Context)
**Core Definition Requirement:**
> "Each Builder maintains its own conversation history..."
> "The user should be able to: [...] continue conversations"
> "continue interacting with each Builder independently"

**Olympics Gap:**
The existing events (e.g., `OPS-BRZ-002` Basic Chat, `OPS-SLV-001` Multi-Tool) test single exchanges or single-request tool chains. There is no event that explicitly tests long-lived multi-turn conversations where the Builder must recall context from earlier in the session.

## 6. Observing Progress
**Core Definition Requirement:**
> "The user should be able to: [...] observe progress"

**Olympics Gap:**
Phase 0 explicitly defers this requirement. The Phase 0 documentation states under "Future Levels — Beyond Phase 0: Platinum: Concurrent long-running workflows across panes with progress reporting." Therefore, this Core Definition requirement currently has no Phase 0 Olympic event.

## 7. Application Responsiveness
**Core Definition Requirement:**
> "The application should remain responsive throughout normal operation."

**Olympics Gap:**
While the Olympics define latency targets (e.g., TTFT < 5 seconds), there is no specific test ensuring that the main application window (UI) remains responsive (i.e., not freezing or blocking) while Builders are performing heavy or concurrent operations in the background.