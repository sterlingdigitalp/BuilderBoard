# JULES_PROJECT_UNDERSTANDING

## 1. What is BuilderBoard?
BuilderBoard exists to allow a software developer to work with four independent AI software engineers simultaneously from a single desktop application. Nothing more. Nothing less. It replaces four separate AI coding assistant windows with one BuilderBoard window to perform engineering work managing four independent software projects simultaneously.

## 2. What is BuilderBoard NOT trying to be in Version 1?
BuilderBoard is explicitly not trying to optimize for:
- Maximizing feature count (features that do not serve four-pane independent operation are out of scope).
- Architectural purity or implementation elegance (implementation details are fungible if the runtime behaves correctly).
- Unit test coverage (tests are a means, not an end).
- Backward compatibility with single-pane tools.
Furthermore, passing tests, clean architecture, or a completed implementation are not substitutes for successful runtime behavior.

## 3. What is the Core Promise?
The Core Promise is: "BuilderBoard exists to allow a single user to accomplish everything possible with one AI software engineering assistant simultaneously across four independent Builder panes." Each Builder operates independently without interfering with one another, maintaining its own repository, conversation history, runtime state, model, tools, and engineering task.

## 4. What are the three highest-priority deficiencies?
Based on the `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md` and `RUNTIME_ENGINEERING_LEDGER.md`, the top three fundamental blockers are:
1. **Builders cannot yet reliably complete general engineering requests:** Repository-wide engineering requests frequently fail, builders exhaust the planner/tool budget, and work stops before completion.
2. **Repository discovery is unreliable:** Builders do not reliably discover repository structure (e.g., counting source files, discovering implementation files, repository-wide inspection).
3. **Tool execution is not yet sufficiently reliable:** There is a significant number of failed tool invocations, repeated validation failures, and planner retries.

*(Note: The `RUNTIME_ENGINEERING_LEDGER.md` aligns with this, noting P0/P1 issues related to repository-scale discovery exhausting the planner, tool validation failures causing planner exhaustion, and hardcoded builder routing blocking multi-pane independence).*

## 5. What engineering work should NOT be performed yet?
No feature, tool, or capability may be added if it would delay or weaken the Core Promise. Specifically:
- **No new features above the current certification level:** The Roadmap Gate strictly forbids implementing any feature unless the runtime is already certified at the level that feature requires.
- **No feature development while regressions exist:** When a regression is detected, all feature development stops until it is resolved.
- **No work on non-goals:** Work should not be performed on maximizing feature count, architectural purity, expanding test coverage merely for the sake of coverage, or supporting backward compatibility for single-pane tools.

## 6. What should every future engineering task optimize for?
Every future engineering task must optimize for successful, reliable runtime behavior that serves the Core Promise. Before every decision, every engineering task must answer the single question:
> **"Does this improve BuilderBoard’s ability to fulfill its Core Promise?"** (or "Does this preserve or advance the Core Promise?")

If the answer is no, it is not part of BuilderBoard Version 1 and should not be performed.