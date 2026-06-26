# Prompt Architecture Audit

## Audit Target
Determine whether Builders are ever explicitly instructed: "When sufficient information exists, stop using tools and answer."

## Findings

I have audited every system prompt, tool prompt, and prompt builder module within the `src-tauri/src` directory, specifically looking at:
- `src-tauri/src/execution/capability_resolver.rs` (builds the tool advertisements and tool instructions)
- `src-tauri/src/stream_execution.rs` (handles the tool calling loop and injects system prompts per round)
- `src-tauri/src/storage/commands.rs` (handles filesystem context enrichment prompts)
- `src-tauri/src/execution/grok_build.rs` (provider prompt building)

I performed an exhaustive search for the string "When sufficient information exists, stop using tools and answer." (and variations thereof, such as "enough information", "stop calling tools", "stop using tools", and "sufficient information").

**Result:** The Builders are **never** explicitly instructed to stop using tools when they have sufficient information in any system or tool prompt in the Rust backend.

### Context from Memory & Documentation
This finding perfectly aligns with `docs/runtime/reports/2026-06-26_BUILDER_T_CONVERGENCE_REPORT.md` (Builder T — Planner Convergence Test Report), which states:

> ### 2. The real issue is *delayed* convergence, not *absent* convergence
>
> BB-0006's title says "planner lacks convergence detection." This is misleading.
> The planner *does* detect convergence (it checks `tool_calls.is_empty()` every
> round). The real problem is that the LLM *chooses* to keep calling tools beyond
> what a human would consider sufficient, because:
>
> - The tool-calling prompt encourages exhaustive tool use.
> - **There is no "I have enough information, here is the answer" signal injected into the system prompt.**
> - Each round *resets* the LLM context window — the model doesn't see its own prior text as a complete answer, only as partial progress.

The report recommends fixing this by:
1. Adding a system prompt directive: `"Once you have gathered enough information to answer the user's request, stop calling tools and provide the final answer."`

This fix has **not** been implemented yet, which explains why the audit yielded no results for this instruction.

## Conclusion
Builders are currently **NOT** explicitly instructed to stop using tools when sufficient information exists. The system relies entirely on the LLM independently choosing to stop emitting ````tool_call```` blocks, without explicit prompting guidance to do so.
