# Builder Lifecycle Summary

## The Builder Lifecycle
The Builder Lifecycle represents the states and transitions of a single independent Pane within BuilderBoard:
1. Builder created -> Repository attached
2. Repository attached -> Conversation created
3. Conversation created -> Prompt
4. Prompt -> Planning
5. Planning -> Tools
6. Tools -> Response
7. Response -> Persistence
8. Persistence -> Idle
9. Idle -> Destroy

## Lifecycle Simplification
During the **Idle -> Destroy** transition, a pane is closed (`pane_close`). Previously, this function updated the pane's status back to `'idle'` while simultaneously marking it as closed (`closed_at`). I removed this redundant `status = 'idle'` update from the SQL query in `src-tauri/src/storage/repositories/panes.rs`.

## Why This is Correct
Once a pane is closed (destroyed), it is logically removed from the user's workspace and the `closed_at` timestamp is set. An inactive, soft-deleted pane does not require an active state identifier like `'idle'`. Updating the status during deletion serves no functional purpose and simply creates unnecessary database write operations.

## Runtime Implications
- Reduced database mutation overhead during pane destruction.
- Ensures destroyed panes maintain their terminal state instead of artificially transitioning to an 'idle' status.
- No impact on active pane operation, as active panes correctly transition to 'idle' at the end of their execution loop.

## Unrelated Test Failure Documentation
During test execution, one pre-existing test failed:
- **Test**: `execution::manager::tests::manager_respects_builder_preference_when_available`
- **Reason**: The test asserts that the `grok` engine is selected as preferred. However, the `grok` CLI is unavailable in the current test environment, which causes its health check to return `"cli missing"`, subsequently causing the manager to fall back to the `"openai"` engine.
- **Isolation**: This failure is purely dependent on external CLI availability and test environment setup. It is completely isolated from the Builder Lifecycle changes in `panes.rs` which solely involve database persistence layers.
