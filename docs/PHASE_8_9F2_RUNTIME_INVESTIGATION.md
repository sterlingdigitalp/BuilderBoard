# Phase 8.9F.2 — Independent Runtime Investigation

**Role:** Independent Verification Engineer (Builder B)  
**Date:** 2026-06-24  
**Target:** UI showing "No Builders Available", "No Engines Available", "No Account", "Execution Failed" while Projects/Models/Effort display

---

## 1. Independent Theory

**Root Cause: The `Promise.all` in `usePaneChat.ts` fails due to a single failing child promise, cascading to destroy ALL four data sources (accounts, messages, engines, builders). Because the state arrays are never populated, the UI renders empty-state options. Meanwhile, `selectedModelId` and `selectedEffort` survive because they are initialized from localStorage (`paneSettingsStore`) — a completely independent data path that is never cleared by the error catch block.**

The symptom matrix maps directly to this theory:

| Symptom | Explanation |
|---|---|
| No Builders Available | `builders[]` stays `[]` — `setBuilders()` never called |
| No Engines Available | `engines[]` stays `[]` — `setEngines()` never called |
| No Account | `accounts[]` stays `[]` — `setAccounts()` never called |
| Execution Failed | `catch(loadError)` → `setDisplayState("error")`, `setError(...)` |
| Models display | `selectedModelId` initialized from `paneSettingsFor()` (localStorage) — survives error |
| Effort displays | `selectedEffort` initialized from `paneSettingsFor()` (localStorage) — survives error |
| Projects load | `project_list` is NOT in the `Promise.all` — it loads via a completely different code path |

---

## 2. Evidence

### Evidence A: `Promise.all` failure cascade

**File:** `usePaneChat.ts:124-129`

```typescript
const [loadedAccounts, loadedMessages, loadedEngines, loadedBuilders] = await Promise.all([
  accountList("openai"),
  messageList(pane.id),
  engineList(),
  builderList()
]);
```

`Promise.all` rejects on the FIRST rejected promise. If ANY one call fails (network error, data validation error, database inconsistency, parameter mismatch), ALL four results are lost. The catch block at line 171 sets error state but never partially populates the arrays:

```typescript
} catch (loadError) {
  if (isActive) {
    setError(errorMessage(loadError));   // banner shows "Execution Failed"
    setDisplayState("error");
  }
}
```

All four `set*` calls (lines 149-152) are inside the `try` block and NEVER execute on failure. Initial state (`useState([])`) persists.

### Evidence B: Model/Effort survive because they come from localStorage, not the backend

**File:** `usePaneChat.ts:82-91`

```typescript
const initialSettings = paneSettingsFor(pane);  // reads from localStorage
const [selectedModelId, setSelectedModelId] = useState(initialSettings.modelId);
const [selectedEffort, setSelectedEffort] = useState<EffortLevel>(initialSettings.effort);
```

These are initialized ONCE from `paneSettingsStore.ts`, which reads from `localStorage` — completely independent of the backend calls in `loadChat`. The `loadChat` catch block never clears these values:

```typescript
// catch block sets displayState + error, but NEVER resets modelId or effort
setError(errorMessage(loadError));
setDisplayState("error");
```

### Evidence C: Empty state rendering matches ChatControls

**File:** `ChatControls.tsx:114-123` (builders), `134-143` (engines), `154-163` (accounts)

```tsx
{builders.length === 0 ? (
  <option value="">No builders</option>
) : (...)}
{engines.length === 0 ? (
  <option value="">No engines</option>
) : (...)}
{accounts.length === 0 ? (
  <option value="">None</option>
) : (...)}
```

When arrays are empty (initial state), these branches render the exact messages the user reports.

### Evidence D: Model/Effort fallback behavior when engines is empty

**File:** `ChatControls.tsx:100-101`

```typescript
const currentEngine = engines.find((e) => e.id === selectedEngineId) || engines[0];
const modelOptions = currentEngine ? currentEngine.models.map(...) : [];
const effortOptions = currentEngine ? currentEngine.supportedEfforts.map(...) : defaultEffortOptions;
```

When `engines = []`, `currentEngine` is `undefined`, so:
- `modelOptions = []` — model select renders but with zero `<option>` elements (disabled)
- `effortOptions = defaultEffortOptions` — effort select renders with Low/Medium/High/Max options (active)

### Evidence E: `messageList` backend validates pane existence AND project_id

**File:** `src-tauri/src/storage/repositories/messages.rs:19`, `src-tauri/src/storage/repositories/panes.rs:165-177`

```rust
pub fn list_for_pane(connection: &Connection, pane_id: &str) -> StorageResult<Vec<MessageDto>> {
    PaneRepository::get_open_by_id(connection, pane_id)?;  // FAILS if pane not found, closed, or missing project_id
    ...
}
```

```rust
pub fn get_open_for_execution(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
    let pane = connection.query_row(..., [pane_id], map_pane_row)?.optional()?
        .ok_or_else(|| StorageError::NotFound(...))?;
    if pane.project_id.is_none() {
        return Err(StorageError::InvalidInput("open pane is missing project_id"));
    }
    Ok(pane)
}
```

This is a POTENTIAL failure point. If a pane was created before migration 0012 (project_id enforcement), or if the backfill failed, or if a pane's project was deleted, `messageList` fails with an error — cascading to kill ALL initialization.

### Evidence F: Projects load independently

Projects are NOT loaded in the `Promise.all` in `loadChat`. They are passed as a prop (`projects: ProjectDto[]`) from the parent component that calls `Pane`.

**File:** `Pane.tsx:10` — `projects: ProjectDto[]` is a prop. The parent loads projects via a separate mechanism (`useProjects` or similar) that is completely independent from `usePaneChat`.

---

## 3. Files Involved

| File | Line(s) | Role |
|---|---|---|
| `src/hooks/usePaneChat.ts` | 82-91 | State initialization from localStorage (survives errors) |
| `src/hooks/usePaneChat.ts` | 124-129 | **`Promise.all` failure cascade** — one failure kills four data sources |
| `src/hooks/usePaneChat.ts` | 149-152 | State setters for accounts/engines/builders — only in `try` block |
| `src/hooks/usePaneChat.ts` | 171-175 | Catch block — sets error + displayState but NEVER resets model/effort |
| `src/components/Chat/ChatControls.tsx` | 100-101 | Engine-dependent fallback — model goes empty, effort uses defaults |
| `src/components/Chat/ChatControls.tsx` | 114-123 | Builder empty state renders "No builders" |
| `src/components/Chat/ChatControls.tsx` | 134-143 | Engine empty state renders "No engines" |
| `src/components/Chat/ChatControls.tsx` | 154-163 | Account empty state renders "None" |
| `src/components/Chat/ChatPane.tsx` | 36-38 | "Execution Failed" banner title |
| `src/components/Pane/Pane.tsx` | 74 | `usePaneChat(pane)` hook call |
| `src/stores/paneSettingsStore.ts` | 82-99 | localStorage-based settings — independent of backend |
| `src/stores/chatCommands.ts` | 183-193 | `messageList` — throws on validation failure |
| `src/stores/accountCommands.ts` | 44-59 | `toAccountDto` — throws on unexpected shape |
| `src-tauri/src/storage/commands.rs` | 352-412 | `stream_chat` — route validation (new code) |
| `src-tauri/src/storage/commands.rs` | 233-242 | `message_list` — calls `list_for_pane` |
| `src-tauri/src/storage/repositories/messages.rs` | 18-34 | `list_for_pane` — validates pane exists + has project_id |
| `src-tauri/src/storage/repositories/panes.rs` | 158-177 | `get_open_by_id` / `get_open_for_execution` — pane validation |

---

## 4. Most Likely Root Cause

**Ranked by likelihood:**

### #1 (Most Likely): Frontend error — single call fails, `Promise.all` kills everything

No error isolation. If ANY of the four parallel calls fails (network glitch, Tauri IPC timeout, data validation error from a stale account), ALL four datasets are lost. The initial empty arrays persist.

**Trigger candidate A:** `accountList("openai")` — `toAccountDto` throws if any account has unexpected `id` or `providerId` shape (e.g., from a partial database migration or corrupt data).

**Trigger candidate B:** `messageList(pane.id)` — Backend `list_for_pane` calls `PaneRepository::get_open_by_id`, which validates the pane exists AND has a non-null `project_id`. If any pane lacks a `project_id` (e.g., created before migration 0011 backfill, or failed backfill), this FAILS.

**Trigger candidate C:** `accountList("openai")` parameter serialization — The frontend passes `{ providerId: "openai" }`. Backend expects `provider_id: Option<String>`. If Tauri's camelCase→snake_case conversion has an edge case with `Option` parameters, the invoke fails.

### #2: Database inconsistency after migration 0012

Migration 0012 enforces `project_id IS NOT NULL` via SQL trigger. But existing panes without project_ids would fail this check. If the backfill (0011) didn't cover all existing panes, `list_for_pane` returns a `StorageError::InvalidInput` for affected panes.

### #3: Race condition with pane creation

If `loadChat` runs before the pane is fully persisted in the database, `messageList(pane.id)` fails because `get_open_by_id` returns NotFound. This is possible if the pane list refresh and `loadChat` race on initial mount.

---

## 5. Alternate Hypotheses

### H1: `selectedBuilderId` stale closure in `sendMessage`

**Evidence:** `sendMessage` uses `selectedBuilderId` at line 397 (`builderId: selectedBuilderId || undefined`) but it is NOT in the dependency array (line 436: `[inputValue, pane.id, reloadMessages, selectedAccountId, selectedModelId, selectedEffort, selectedEngineId]`).

**Rating:** LOW. This would not cause the initial load failure. It would cause the wrong `builderId` to be sent during execution, which the backend route validation might reject — but that's a secondary send-time error, not the initial mount failure.

### H2: Backend `stream_chat` route validation rejects valid requests

**Evidence:** New route validation at `storage/commands.rs:367-383` checks `global_engine_registry().get(&provider_id).is_some()` and builder registry lookup.

**Rating:** LOW. Provider IDs are "openai" or "grok" — both registered engines. This would be a send-time error, not an initial load error.

### H3: `engineList()` or `builderList()` fails

**Evidence:** These return raw `Vec<serde_json::Value>` with no validation. Extremely unlikely to fail.

**Rating:** VERY LOW. These commands have no database dependencies, no state injection, and return simple JSON. They only fail if Tauri itself fails.

### H4: Toast/notification system showing stale error

**Rating:** MEDIUM. If `loadChat` succeeded once but a subsequent `reloadMessages` call failed, the error state would persist while engines/builders/accounts remain populated. The user's symptom requires ALL three to be empty, so this doesn't match.

---

## 6. Confidence

**Confidence in root cause theory: 85/100**

| Component | Confidence | Rationale |
|---|---|---|
| `Promise.all` cascade is the mechanism | 100% | Direct code evidence at `usePaneChat.ts:124-129` |
| Model/effort survive from localStorage | 100% | Direct code evidence at `usePaneChat.ts:82-91` and `paneSettingsStore.ts` |
| Empty state rendering is correct | 100% | Direct code evidence at `ChatControls.tsx:114-163` |
| Backend validation exists for pane+project_id | 95% | Direct code at `messages.rs:19`, `panes.rs:165-177` |
| Which specific call fails | 70% | Need database state to confirm exact failure |
| Database inconsistency exists | 60% | Migration 0012 enforcement is strong — panes should have project_ids |

---

## 7. Agreement with Builder C

Builder C's findings are not yet available. Based on the evidence I've found, I would:

**Agree** that the runtime state is inconsistent (multiple evidence paths confirm).
**Agree** that engine/builder/account lists are all empty while model/effort display.
**Disagree** if Builder C's theory focuses on the backend `stream_chat` route validation — that's a send-time path, not the initial load path.
**Agree** if Builder C identifies the `Promise.all` anti-pattern as the mechanism.
**Disagree** if Builder C says the engines/builders registries are empty — the registries return correct data; the failure is in how the frontend loads them.

---

## 8. Fix Recommendations (not implement, just state)

These are recommendations for Builder C or the implementation team:

1. **Isolate data loading** — Replace `Promise.all` with individual load calls, each with its own error handling. A failure in `accountList` should not prevent engines/builders/messages from loading:

   ```typescript
   const [loadedAccounts, loadedMessages, loadedEngines, loadedBuilders] = await Promise.allSettled([
     accountList("openai").catch(() => []),  // graceful degradation
     messageList(pane.id).catch(() => []),
     engineList(),  // these never fail
     builderList(), // these never fail
   ]);
   ```

2. **Add error isolation at the backend** — If `list_for_pane` fails, return an empty list with a warning, not an error. The pane validation in `list_for_pane` (`get_open_by_id` check) is redundant with the pane existence invariant.

3. **Fix `sendMessage` dependency array** — Add `selectedBuilderId` to the dependency array at line 436:

   ```typescript
   const sendMessage = useCallback(async () => {
     ...
   }, [inputValue, pane.id, reloadMessages, selectedAccountId, selectedModelId, selectedEffort, selectedEngineId, selectedBuilderId]);
   ```

---

## 9. Summary

| Item | Finding |
|---|---|
| **Document** | `docs/PHASE_8_9F2_RUNTIME_INVESTIGATION.md` |
| **Root cause** | `Promise.all` failure cascade — single API call failure destroys all four data sources |
| **Specific trigger** | Likely `accountList("openai")` or `messageList(pane.id)` — both perform runtime data validation |
| **Why model/effort survive** | Independent localStorage path (`paneSettingsStore`), never cleared by error handler |
| **Why projects load** | Separate loading mechanism (parent component, not in `Promise.all`) |
| **Confidence** | 85/100 |
| **Fix priority** | HIGH — `Promise.allSettled` or individual error handling |
| **Secondary findings** | `sendMessage` stale closure on `selectedBuilderId` (MEDIUM); `list_for_pane` pane validation may reject valid panes (LOW); no error isolation in data loading (HIGH) |

---

## PRINT FINAL
