# Core Promise

*The single reason BuilderBoard exists.*

---

## Mission

BuilderBoard exists to allow a user to accomplish everything they can accomplish with a single AI software engineering assistant, simultaneously across four independent Builder panes.

## The Core Promise

> **BuilderBoard exists to allow a single user to accomplish everything possible with one AI software engineering assistant simultaneously across four independent Builder panes.**

Everything in the project — every feature, every fix, every refactor, every tool — serves this promise. Any feature that does not serve this promise is out of scope. Any feature that weakens this promise is a regression.

## What Success Looks Like

A user opens BuilderBoard and sees four Builder panes. In each pane, they can:

- Send a message describing engineering work to be done.
- Receive a coherent, context-aware response.
- Request tool execution (read files, run shell commands, search code, check git status).
- Receive tool results inline in the conversation.
- Chain multiple tool calls in a single exchange.
- Complete the work without the loop exhausting or failing.

All four panes operate simultaneously without interference. Each has its own conversation history, tool execution context, and model state.

## What Failure Looks Like

The Core Promise has failed if:

- A pane crashes, freezes, or becomes unresponsive.
- Tool calls do not execute or return errors.
- The tool loop exhausts without producing a final response.
- Panes interfere with each other (cross-conversation leakage, shared state corruption).
- The user must restart the application to recover from any runtime state.

## Non-Goals

The following are explicitly not part of the Core Promise:

- **Maximizing feature count.** A feature that does not serve four-pane independent operation is lower priority than fixing a runtime regression.
- **Architectural purity.** The implementation is fungible — the runtime behavior is what matters.
- **Unit test coverage.** Tests are a means, not an end. They exist to protect the Core Promise.
- **Backward compatibility with single-pane tools.** BuilderBoard is a multi-pane system from the ground up.

## The Single Question

Before every decision, ask:

> Does this preserve or advance the Core Promise?

If the answer is No, the decision must be reconsidered. If the answer is I don't know, more analysis is needed before proceeding.
