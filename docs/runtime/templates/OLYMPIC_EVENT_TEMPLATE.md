# Olympic Event Template

Use this template to define new Runtime Olympic events.

---

## Event Metadata

| Field | Value |
|-------|-------|
| **Event ID** | `OPS-{TIER}-{NNN}` |
| **Name** | {Human-readable name} |
| **Tier** | Bronze / Silver / Gold / Platinum / Diamond / Legendary |
| **Created** | {YYYY-MM-DD} |
| **Author** | {Name} |

---

## Event Definition

### Mission

{What the user is trying to accomplish. One to two sentences.}

*Example: Ask the assistant to read a specific file and return its contents.*

### Prerequisites

{Any conditions that must be met before this event can be executed.}

*Example: The application must be running with a configured OpenAI provider.*

### User Action

{Exactly what the user does. Step by step.}

*Example:*
1. *Open a Builder pane.*
2. *Type "read src/main.rs" in the input field.*
3. *Press Enter.*

### Expected User Outcome

{What the user sees and experiences. Concrete and observable.}

*Example: The assistant responds with the contents of src/main.rs. No error messages appear.*

---

## Pass Criteria

{Conditions that must all be true for the event to pass.}

Condition | Measurement Method
----------|-------------------
{condition 1} | {how to check}
{condition 2} | {how to check}
{condition 3} | {how to check}

*Example:*
| Condition | Measurement Method |
|-----------|-------------------|
| Tool `filesystem.read` is called | Inspect runtime logs for tool invocation event |
| Tool executes without error | Inspect runtime logs — no ToolFailed event |
| File contents appear in response | Response text contains expected content |
| Loop terminates within 2 rounds | Inspect runtime logs — final round has no tool calls |

---

## Latency Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Time to first token | < {N} seconds | |
| Total response time | < {N} seconds | |
| Tool execution time | < {N} seconds | |

---

## Metrics Collected

| Metric | Type | Description |
|--------|------|-------------|
| `{metric_name}` | duration / count / boolean | {description} |

*Example:*
| Metric | Type | Description |
|--------|------|-------------|
| `tool_rounds` | count | Number of tool call rounds in the loop |
| `tool_execution_ms` | duration | Time from tool start to tool finish |
| `tool_success` | boolean | Whether the tool completed without error |
| `ttft_ms` | duration | Time to first token |

---

## Regression Linkage

{References to related issues, PRs, or prior certification events.}

*Example: Phase 9A.5 tool loop fix (PR #xxx).*

---

## Certification Weight

{Percentage contribution to overall certification score.}

*Example: 10%*

---

## Notes

{Any additional information for the tester.}

*Example: This event requires a file to exist at the specified path. The test project should contain src/main.rs.*
