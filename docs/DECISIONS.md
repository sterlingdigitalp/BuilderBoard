## 2026-06-23

### Frontend Framework

Decision:
React + TypeScript + Vite

Reason:
Fast iteration, Tauri compatibility

Status:
Accepted

---

### Desktop Framework

Decision:
Tauri 2.x

Reason:
Native desktop app with Rust backend

Status:
Accepted

---

### Persistence

Decision:
SQLite

Reason:
Local-first architecture

Status:
Accepted

---

### Secret Storage

Decision:
OS Keychain

Reason:
No secrets in SQLite

Status:
Accepted

---

### Provider Abstraction

Decision:
LLMProvider trait

Reason:
Provider-independent architecture

Status:
Accepted

---

### Phase Discipline

Decision:
No feature implementation outside assigned phase

Reason:
Prevent scope creep

Status:
Accepted

---

### Phase 2A Storage Layout

Decision:
Single `storage` module with embedded migrations via `include_str!`

Reason:
Reliable migration loading in dev and production bundles

Status:
Accepted

---

### Phase 2A Provider Seeds

Decision:
Seed only anthropic, openai, google in `0001_initial_schema`

Reason:
Match Phase 2A scope; additional providers deferred

Status:
Accepted