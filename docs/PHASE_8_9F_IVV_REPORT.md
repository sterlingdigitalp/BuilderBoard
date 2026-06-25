# Phase 8.9F — Independent Verification & Validation Report

**Role:** Independent Verification Engineer  
**Audit Date:** 2026-06-24  
**Target:** Phase 8.9A–8.9E implementation  

---

## 1. Executive Summary

**Builder C's implementation is architecturally sound but has one critical defect and several medium-severity issues. Merge blocked until the critical defect is resolved.**

The Phase 8.9 implementation creates a generalized execution engine architecture (8.9A), a reusable CLI process engine with Grok integration (8.9B), dynamic engine/model/effort discovery in the UI (8.9C), a Builder registry with execution policies (8.9D), and an intelligent Execution Manager for routing (8.9E). The architecture documents (8.9A–8.9D) are thorough and well-researched.

**However, the core orchestration path in `stream_execution.rs` has a critical design defect:** builder names are hardcoded in the routing logic rather than resolved generically through the `ExecutionManager`. Adding a new builder requires modifying `stream_execution.rs`. This violates the 8.9A principle of dynamic, registry-driven execution.

**Additionally:** 6 dead functions, 1 debug `eprintln!` in production path, 1 unused variable, and significant test coverage gaps in the execution module.

**Overall Confidence Score: 72/100**  
**Production Readiness Score: 55/100**

---

## 2. Validation Matrix

| Check | Result | Evidence |
|---|---|---|
| `cargo check` | PASS (17 warnings) | All warnings are pre-existing dead code + unused variable (none new from Phase 8.9) |
| `cargo test` | PASS (104/104 unit, 31/31 integration, 3/3 doc) | All tests pass. 7 ignored (require live credentials / network) |
| `npm run typecheck` | PASS (0 errors) | TypeScript check clean |
| **Total tests** | 138 pass, 0 fail, 7 ignored | |

### Test Coverage Analysis

| Module | Tests | Coverage Quality |
|---|---|---|
| `execution/engine.rs` | 2 unit | Basic — capabilites and registry existence only |
| `execution/manager.rs` | 3 unit | Good — tests builder preference, fallback, class roundtrip |
| `execution/cli.rs` | 0 | **Missing** — no tests for process spawning, streaming, cancellation |
| `execution/grok_build.rs` | 0 | **Missing** — no tests for prompt building, event parsing, CLI availability |
| `execution/context.rs` | 0 | **Missing** |
| `execution/event.rs` | 0 | **Missing** |
| `execution/request.rs` | 0 | **Missing** |
| `execution/capabilities.rs` | 0 | **Missing** |
| `builders/mod.rs` | 0 | **Missing** — no tests for BuilderRegistry, registration, resolution |
| `storage/commands.rs` (`engine_list`, `builder_list`, `resolve_execution`) | 0 | **Missing** — no Tauri command tests |
| `stream_execution.rs` (new routing) | 0 | **Missing** — no tests for ExecutionManager integration path |

**Coverage decision:** Insufficient. The core execution module has only 5 tests. CLI/Grok/Builder subsystems have zero. Acceptable for a prototype, NOT acceptable for production.

---

## 3. Architecture Compliance Matrix

### 8.9A — ExecutionEngine Abstraction

| Requirement | Status | Finding |
|---|---|---|
| ExecutionEngine boundary exists | ✓ PASS | `execution/engine.rs:42` — `ExecutionEngine` trait defined with `execute()`, `supports()`, capabilities, models, health |
| OpenAI fully encapsulated | ✓ PASS | `OpenAIExecutionEngine` at `engine.rs:97` wraps old `OpenAIProvider` streaming logic; no OpenAI types leak outside |
| ExecutionRequest generalized | ✓ PASS | `request.rs:65` — 7 variants: Chat, Completion, Embed, Image, Tool, Structured, Raw |
| ExecutionContext generalized | ✓ PASS | `context.rs:37` — fields for project, filesystem, environment, policy, cancellation, optional credentials |
| ExecutionEvent generalized | ✓ PASS | `event.rs:14` — 13 variants covering chat, reasoning, tools, artifacts, progress, errors, cancellation, metadata |
| No OpenAI-centric orchestration outside engine | △ PARTIAL | `stream_execution.rs:197` — `openai_span` and `OPENAI_REQUEST_DURATION_MS` metric names retain OpenAI naming even when Grok is the engine |

### 8.9A.1 — Execution Generalization

| Requirement | Status | Finding |
|---|---|---|
| ExecutionContext not credential-centric | ✓ PASS | `context.rs:59` — `credential: Option<CredentialHandle>` — optional, not required |
| ExecutionRequest supports generalized execution | ✓ PASS | `request.rs:65` — polymorphic enum with variant-specific structs |
| ExecutionEvent sufficiently generic | ✓ PASS | `event.rs:14` — 13 variants covering all execution modalities |
| EngineCapabilities support future engines | ✓ PASS | `capabilities.rs:64` — locality, transports, features (13 booleans), context limits, resources, tags |

### 8.9B — CLIExecutionEngine + Grok Build

| Requirement | Status | Finding |
|---|---|---|
| CLIExecutionEngine is reusable | ✓ PASS | `cli.rs:41` — no Grok-specific code; generic `CLIProcessConfig` + `run_and_stream_events()` with parser hook |
| GrokBuildExecutionEngine contains only Grok-specific logic | ✓ PASS | `grok_build.rs:36` — only prompt building, CWD resolution, Grok event parsing, CLI command construction |
| Process management centralized | ✓ PASS | All process lifecycle (spawn, stream, cancel, timeout, cleanup) is in `cli.rs` |
| Grok uses same execution path as OpenAI | ✓ PASS | Both implement `ExecutionEngine` trait; both called via `engine.execute()` in `stream_execution.rs:259` |
| Cancellation and cleanup exist | ✓ PASS | `cli.rs:113-118` — AtomicBool cancellation with `child.kill()`; `cli.rs:141-157` — timeout with kill; `cli.rs:160-164` — final cleanup |
| Registry integration correct | ✓ PASS | `engine.rs:289-290` — Grok registered as `"grok"` alongside `"openai"` |

### 8.9C — Execution UX

| Requirement | Status | Finding |
|---|---|---|
| Engine discovery is dynamic | ✓ PASS | `engine_list` Tauri command; `engineCommands.ts` frontend; dynamically populated dropdown |
| Model discovery is dynamic | ✓ PASS | Per-engine `list_models()`; models populated from selected engine |
| Effort selection is dynamic | ✓ PASS | Per-engine `supported_effort_levels()`; efforts populated from selected engine |
| No hardcoded engine lists remain | △ PARTIAL | `ChatControls.tsx:32-38` — hardcoded grid layout assumes 5 columns (builder, engine, account, model, effort); effort dropdown is last |
| UI is driven from registries | △ PARTIAL | Engine/builder/model/effort dropdowns are registry-driven. BUT: no tool call rendering, no status bars, no permission UX, no diff preview, no stop button — per 8.9C architecture doc |

### 8.9D — Builder System

| Requirement | Status | Finding |
|---|---|---|
| BuilderRegistry exists | ✓ PASS | `builders/mod.rs:60` — `BuilderRegistry` with `register()`, `get()`, `list()`, `list_names()` |
| Builder definitions load correctly | △ PARTIAL | Builders are **hardcoded** in `register_default_builders()` at `builders/mod.rs:100-148`. No YAML loading. Comment: "In full impl, these would be loaded from .builderboard/builders/*.yaml" |
| Builder selection drives execution | ✓ PASS | `usePaneChat.ts:175-189` — `selectBuilder()` applies preferred engine, model, effort to pane settings |
| Builder does not directly execute engines | ✓ PASS | Builder selection → ExecutionManager → engine resolve — builder never calls engine directly |
| Builder runtime profile is correct | ✓ PASS | `manager.rs:82-95` — `From<&Builder> for ExecutionProfile` converts builder policy → manager profile |
| Manual override still works | ✓ PASS | User can independently change engine/model/effort after builder selection; `paneSettingsStore.ts` persists per-pane overrides |
| Builder fallback behaves correctly | △ PARTIAL | `ExecutionManager::resolve()` uses fallback_engines. Only Builder C defines fallback (grok→openai). A/B have empty fallback lists |

### 8.9E — Execution Manager

| Requirement | Status | Finding |
|---|---|---|
| ExecutionClass exists | ✓ PASS | `manager.rs:19` — 10 variants with `from_str()` and `as_str()` |
| ExecutionProfile exists | ✓ PASS | `manager.rs:72` — class, preferred_engine, fallback_engines, effort, default_model, review, memory |
| ExecutionResolution exists | ✓ PASS | `manager.rs:99` — engine_id, model, effort, reason, class, policy_applied |
| ExecutionManager is routing authority | ✓ PASS | `manager.rs:178` — `resolve()` called from `stream_execution.rs:148` for builder paths |
| Engine scoring is deterministic | ✓ PASS | `score_engine()` at `manager.rs:109` — pure function, no randomness, same inputs → same outputs |
| Intelligent fallback works | ✓ PASS | Candidate list: preferred → fallbacks → all available → OpenAI emergency fallback |
| Routing reasons generated | ✓ PASS | `manager.rs:256-262` — three reason templates (preferred, fallback, best available) |
| Policy hooks exist | ✓ PASS | `check_policy()` at `manager.rs:298` — checks chat/streaming capability |
| ExecutionManager integrated into stream_execution.rs | ✓ PASS | `stream_execution.rs:133-176` — full integration |
| Legacy execution paths remain safe | ✓ PASS | `stream_execution.rs:156-175` — direct grok and direct provider/model paths still work unchanged |

---

## 4. Missing Implementation

| Item | Phase | Priority | Impact |
|---|---|---|---|
| `BUILDER.yaml` filesystem loading | 8.9D | HIGH | Builders are hardcoded; users cannot define custom builders. The architecture specifies `.builderboard/builders/*.yaml` loading |
| YAML schema validation for BUILDER.yaml | 8.9D | MEDIUM | `serde_yaml` is in Cargo.toml but no YAML loader exists |
| Builder inheritance resolution | 8.9D | MEDIUM | `extends` field defined in 8.9D doc but not implemented |
| Workflow/graph execution engine | 8.9D | LOW | Workflow steps/conditions/handoffs defined in architecture but not implemented |
| Tool call display UI | 8.9C | MEDIUM | Architecture doc specifies ToolCallCard/DiffPreview/StatusBar components; none exist |
| Permission UX (allow/deny/ask/session) | 8.9C | MEDIUM | Tool permission modes defined but no frontend permission prompts |
| ExecutionUI components (status bar, cost display) | 8.9C | LOW | Specified in architecture but not implemented |
| CLIExecutionEngine health check extensibility | 8.9B | LOW | `health()` on Grok checks CLI binary; other CLI engines would need their own |

---

## 5. Incorrect Implementation

| Item | Phase | Severity | Description |
|---|---|---|---|
| **Hardcoded builder routing in stream_execution.rs** | 8.9A/E | **CRITICAL** | `stream_execution.rs:136` — Builder names `"builder-a"`, `"builder-b"`, `"builder-c"` are hardcoded as string comparisons against `job.provider_id`. Frontend sends builder name as `providerId` in `streamChat`. This means: (1) adding a new builder requires modifying stream_execution.rs, (2) the frontend abuses the `providerId` field to carry builder identity, (3) the ExecutionManager resolve is bypassed for non-builder paths (line 156-175). The architecture intended ExecutionManager to be the sole routing authority for ALL paths. |
| **eprintln! debug print** | 8.9E | MEDIUM | `stream_execution.rs:153` — `eprintln!("[ExecutionManager] Builder=...")` emits to stderr on every execution. Should use `trace_runtime_phase` or similar. |
| **OpenAI metric naming for Grok execution** | 8.9A | LOW | `stream_execution.rs:197,288` — `openai_span`, `OPENAI_REQUEST_DURATION_MS`, `OPENAI_STREAM_TOTAL_MS` are used regardless of which engine is actually executing. These should be generic (`execution_duration_ms`) or engine-parameterized. |
| **Input `providerId` vs engine routing mismatch** | 8.9D | MEDIUM | Frontend: `sendMessage()` sends `selectedEngineId` as `providerId` in `streamChat` (`chatCommands.ts`, `usePaneChat.ts:362`). Backend: `stream_execution.rs` interprets this field as both builder name and engine ID depending on context. The data model conflates provider/engine/builder. |

---

## 6. Technical Debt

| Item | File | Line | Severity | Description |
|---|---|---|---|---|
| Unused variable `mgr` | `storage/commands.rs` | 112 | LOW | `ExecutionManager::new()` result never used; `resolve()` called as static method |
| Unused function `run_filesystem_enrichment_async` | `storage/commands.rs` | 524 | LOW | Dead code, pre-existing |
| Unused function `prepare_filesystem_enrichment` | `storage/commands.rs` | 570 | LOW | Dead code, pre-existing |
| Unused function `apply_stream_chunk` | `storage/commands.rs` | 608 | LOW | Dead code, pre-existing |
| Unused function `conversation_with_filesystem_tool_results` | `storage/commands.rs` | 669 | LOW | Dead code, pre-existing |
| Unused function `trace_project_root_lookup` | `storage/commands.rs` | 815 | LOW | Dead code, pre-existing |
| Unused field `pane_id` | `stream_persistence.rs` | 37 | LOW | Dead field in `PersistEnvelope` |
| Stub trait only | `sidecar/mod.rs` | 1-3 | LOW | 3-line trait with no implementations. Noted in critical context as CLITarget |

---

## 7. Dead Code

| Function/Field | File | Line | Evidence |
|---|---|---|---|
| `run_filesystem_enrichment_async` | `storage/commands.rs` | 524 | `cargo check` warning: "never used" |
| `prepare_filesystem_enrichment` | `storage/commands.rs` | 570 | `cargo check` warning: "never used" |
| `apply_stream_chunk` | `storage/commands.rs` | 608 | `cargo check` warning: "never used" |
| `conversation_with_filesystem_tool_results` | `storage/commands.rs` | 669 | `cargo check` warning: "never used" |
| `trace_project_root_lookup` | `storage/commands.rs` | 815 | `cargo check` warning: "never used" |
| `PersistEnvelope.pane_id` | `stream_persistence.rs` | 37 | `cargo check` warning: "never read" |
| `mgr` | `storage/commands.rs` | 112 | `cargo check` warning: "unused variable" |

**All dead code is pre-existing (not introduced by Phase 8.9).** The Phase 8.9 implementation itself adds no dead code.

---

## 8. Remaining Risks

### CRITICAL

| Risk | Evidence | Mitigation |
|---|---|---|
| **Builder routing bypasses ExecutionManager for new builders** | `stream_execution.rs:136` — hardcoded `"builder-a"`, `"builder-b"`, `"builder-c"` string matching. If a new builder `"builder-d"` is registered, it defaults to the else branch (line 166) which treats the builder name as a direct `engine_id`. This silently falls back to treating the builder as an engine name. | Remove hardcoded builder names. Use `global_builder_registry().get(job.provider_id)` to check if the provider_id is a builder. If yes, route through `ExecutionManager::resolve()`. If not, it's a direct engine selection. |

### HIGH

| Risk | Evidence | Mitigation |
|---|---|---|
| **Grok CLI unavailable at runtime is silent failure** | `grok_build.rs:247-256` — `health()` returns `"cli missing"` but `stream_execution.rs` never checks engine health before routing. If Grok is selected but `grok` binary is absent, the `execute()` call will fail with a spawn error. | Add health check to stream_execution routing path; fallback to next engine if preferred is unhealthy. |
| **No YAML loading means builders are not extensible** | `builders/mod.rs:146` — comment: "In full impl, these would be loaded from .builderboard/builders/*.yaml". No loader exists. | Implement YAML loader before Phase 9B. |

### MEDIUM

| Risk | Evidence | Mitigation |
|---|---|---|
| **eprintln! leaks diagnostic info on every execution** | `stream_execution.rs:153` — debug print to stderr. In production, this creates noise and may expose internal routing details. | Replace with `trace_runtime_phase()`. |
| **Test coverage gap in execution module** | 5 tests for 7 new files. No tests for CLI, Grok, Builder, Context, Event, Request, Capabilities | Add unit tests for CLI process lifecycle, Grok event parsing, Builder registration/resolution, execution path orchestration. |
| **OpenAI metrics mislabel Grok execution** | `stream_execution.rs:197,288` — `OPENAI_REQUEST_DURATION_MS` label used for all engines | Parameterize metric names by engine_id. |

### LOW

| Risk | Evidence | Mitigation |
|---|---|---|
| **Phase 8.9C UX unimplemented** | No tool call displays, status bars, permission prompts, diff preview | Deferred to Phase 9C as per architecture plan. Acceptable. |
| **sidecar/mod.rs is still a stub** | 3-line trait, no implementations | Not blocking; CLIExecutionEngine handles all current CLI needs. |
| **Tool use capability = false for OpenAI** | `capabilities.rs:99` — `tool_use: false` | Currently correct (no tool execution in OpenAI path). Will need update when tool support lands. |

---

## 9. Confidence Score

**72/100**

| Category | Score | Rationale |
|---|---|---|
| Architecture match | 78 | One critical defect (hardcoded builder names). Otherwise clean. |
| Implementation completeness | 65 | Builders hardcoded (no YAML), UX unimplemented, sidecar stub. Core engine trait is solid. |
| Test coverage | 40 | 5 tests for 7 new files in the execution module. CLI, Grok, Builder subsystems untested. |
| Code quality | 80 | Clean code, good comments, consistent naming. One debug print, one unused variable. |
| Production readiness | 55 | Critical defect blocks merge. Medium issues acceptable for follow-up. |

---

## 10. Production Readiness Score

**55/100** — Not production-ready.

**Merge blockers:**
1. CRITICAL: Hardcoded builder routing at `stream_execution.rs:136`
2. HIGH: Grok CLI absence causes runtime failure (no health check in routing path)

**Requires before production:**
3. HIGH: YAML builder loading
4. MEDIUM: Test coverage for execution module
5. MEDIUM: Replace `eprintln!` with proper tracing

---

## 11. Recommendation

**🚫 BLOCK MERGE** — Critical defect found in `stream_execution.rs:136`. The builder routing logic is hardcoded and bypasses the ExecutionManager.

### Required Fixes (merge blockers)

1. **Fix `stream_execution.rs:136`** — Replace hardcoded builder name matching with generic registry lookup:

   ```rust
   // Instead of:
   if job.provider_id == "builder-a" || job.provider_id == "builder-b" ...
   
   // Use:
   let is_builder = global_builder_registry().get(&job.provider_id).is_some();
   if is_builder {
       let class = derive_class(&job.model_id);
       let res = ExecutionManager::resolve(Some(&job.provider_id), Some(class), ...);
       ...
   } else if job.model_id.to_lowercase().contains("grok") || ... {
       // Direct grok model path
   } else {
       // Legacy direct provider/model path
   }
   ```

2. **Add health check to routing** — Before executing on resolved engine, verify `engine.health() == "available"`. If not, fallback to next candidate.

### Recommended Follow-up Items (merge after fixes)

3. **Implement BUILDER.yaml loader** in `builders/mod.rs` using `serde_yaml` (already in Cargo.toml)
4. **Add unit tests** for `cli.rs`, `grok_build.rs`, `builders/mod.rs`, `context.rs`, `event.rs`, `request.rs`, `capabilities.rs`
5. **Replace `eprintln!`** at `stream_execution.rs:153` with `trace_runtime_phase()`
6. **Parameterize metric names** at `stream_execution.rs:197,288` by engine ID
7. **Remove unused variable** at `storage/commands.rs:112`

### Acceptance Criteria for Re-review

- [ ] Fix #1 implemented: builder routing is generic, not hardcoded
- [ ] Fix #2 implemented: health check in routing path
- [ ] `cargo check` has zero new warnings
- [ ] `cargo test` 104/104 unit + 31/31 integration + 3/3 doc tests pass
- [ ] `npm run typecheck` passes
- [ ] Manual test: select Builder C from UI with `grok` CLI installed → routing selects grok engine
- [ ] Manual test: select Builder C from UI with `grok` CLI missing → routing falls back to OpenAI

---

## Appendix: Files Audited

| File | Lines | Phase | Role |
|---|---|---|---|
| `src-tauri/src/execution/mod.rs` | 26 | 8.9A | Module root + re-exports |
| `src-tauri/src/execution/engine.rs` | 313 | 8.9A | ExecutionEngine trait + OpenAI adapter + EngineRegistry |
| `src-tauri/src/execution/request.rs` | 96 | 8.9A.1 | Polymorphic ExecutionRequest |
| `src-tauri/src/execution/context.rs` | 119 | 8.9A.1 | Generalized ExecutionContext |
| `src-tauri/src/execution/event.rs` | 110 | 8.9A.1 | Generalized ExecutionEvent |
| `src-tauri/src/execution/capabilities.rs` | 119 | 8.9A.1 | EngineCapabilities |
| `src-tauri/src/execution/cli.rs` | 168 | 8.9B | Reusable CLIExecutionEngine |
| `src-tauri/src/execution/grok_build.rs` | 360 | 8.9B | GrokBuildExecutionEngine |
| `src-tauri/src/execution/manager.rs` | 367 | 8.9E | ExecutionManager + scoring + resolution |
| `src-tauri/src/builders/mod.rs` | 160 | 8.9D | Builder + BuilderRegistry + hardcoded defaults |
| `src-tauri/src/stream_execution.rs` | 330 | 8.9E | Integration: routing + execution orchestration |
| `src-tauri/src/storage/commands.rs` | 130 (partial) | 8.9C/D/E | engine_list, builder_list, resolve_execution Tauri commands |
| `src-tauri/src/storage/mod.rs` | 361 | — | Tauri command registration |
| `src/stores/engineCommands.ts` | 35 | 8.9C | Frontend EngineInfo + engineList() |
| `src/stores/builderCommands.ts` | 20 | 8.9D | Frontend BuilderInfo + builderList() |
| `src/stores/paneSettingsStore.ts` | 119 | 8.9C | Per-pane engine/model/effort persistence |
| `src/stores/chatCommands.ts` | 292 | — | streamChat invoke (uses selectedEngineId as providerId) |
| `src/hooks/usePaneChat.ts` | 425 | 8.9C/D | Builder/engine/model/effort selection + stream event handling |
| `src/components/Chat/ChatControls.tsx` | 230 | 8.9C | Builder/engine/account/model/effort dropdown UI |
| `src/types/paneSettings.ts` | 24 | 8.9C | Type definitions |
| `docs/PHASE_8_9A_EXECUTION_ARCHITECTURE.md` | 471 | 8.9A | Architecture document |
| `docs/PHASE_8_9B_CLI_EXECUTION_ARCHITECTURE.md` | 846 | 8.9B | Architecture document |
| `docs/PHASE_8_9C_ENGINE_UX_ARCHITECTURE.md` | 642 | 8.9C | Architecture document |
| `docs/PHASE_8_9D_AGENT_CONFIG_ARCHITECTURE.md` | 728 | 8.9D | Architecture document |
| `docs/PHASE_8_9F_IVV_REPORT.md` | this file | 8.9F | This audit report |
| `SKILL_SPEC_v1.1.md` | 1388 | — | Builder definition reference |

---

## PRINT FINAL
