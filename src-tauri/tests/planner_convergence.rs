//! Planner Convergence Olympic Tests for BB-0006.
//!
//! Tests the convergence detection algorithm independently of the HTTP provider
//! layer.  The real planner loop (stream_execution.rs) takes this same shape:
//!
//!   for round in 0..max_tool_rounds {
//!       let response = engine.execute(conversation).await;
//!       let tool_calls = parse_tool_calls(response);
//!       if tool_calls.is_empty() {
//!           // CONVERGED — planner stops
//!           return Ok(response);
//!       }
//!       for tc in tool_calls {
//!           let result = tool.execute(tc).await;
//!           conversation.push(result);
//!       }
//!   }
//!   // MAX ROUNDS — non-convergence fallthrough
//!   Err("Maximum number of tool call rounds reached")
//!
//! Three scenarios:
//!   1. Immediate convergence  — LLM returns text, no tool calls           → 1 round
//!   2. Multi-round convergence — tool calls → final text                  → N+1 rounds
//!   3. Non-convergence        — LLM always returns tool calls             → max_rounds
//!
//! These are pure-logic tests (no network, no provider).  They validate the
//! convergence algorithm shape that the planner must implement.

use serde_json::json;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A simulated LLM response.
#[derive(Clone, Debug)]
enum LlmResponse {
    /// LLM returned text (no tool calls) — convergence signal.
    Text(String),
    /// LLM returned a tool call.
    ToolCall(String, serde_json::Value),
}

/// Result of a convergence simulation.
#[derive(Debug, PartialEq)]
struct ConvergenceResult {
    /// Number of rounds the loop ran (1 = immediate, max_rounds = exhausted).
    rounds_used: u32,
    /// Whether the loop converged (found a Text response) before max_rounds.
    converged: bool,
    /// The final text, if converged.
    final_text: Option<String>,
}

// ---------------------------------------------------------------------------
/// Simulates the planner convergence loop.
///
/// Takes a pre-planned sequence of LLM responses and a max-rounds limit.
/// Returns (rounds_used, converged, final_text).
fn simulate_convergence(
    responses: Vec<LlmResponse>,
    max_rounds: u32,
) -> ConvergenceResult {
    let mut rounds_used: u32 = 0;
    let mut response_index = 0usize;

    for round in 0..max_rounds {
        rounds_used = round + 1;

        // Get the next response (or repeat the last one if we run out)
        let response = if response_index < responses.len() {
            let r = responses[response_index].clone();
            response_index += 1;
            r
        } else {
            // If we run out of planned responses, the planner would get an error.
            // This is a test guard to detect unplanned calls.
            panic!(
                "planner made {} requests but only {} responses were planned",
                rounds_used,
                responses.len()
            );
        };

        match response {
            LlmResponse::Text(text) => {
                // CONVERGENCE DETECTED
                return ConvergenceResult {
                    rounds_used,
                    converged: true,
                    final_text: Some(text),
                };
            }
            LlmResponse::ToolCall(name, arguments) => {
                // In the real loop, we'd execute the tool and add the result
                // to the conversation before looping.  We skip that here
                // because the LLM responses are pre-planned.
                let _ = (name, arguments);
            }
        }
    }

    // MAX ROUNDS REACHED — non-convergence
    ConvergenceResult {
        rounds_used,
        converged: false,
        final_text: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn convergent_planner_terminates_on_first_no_tool_response() {
    let result = simulate_convergence(
        vec![LlmResponse::Text("Hello from the assistant".to_string())],
        10,
    );

    assert!(result.converged, "should converge");
    assert_eq!(result.rounds_used, 1, "1 round");
    assert_eq!(
        result.final_text.as_deref(),
        Some("Hello from the assistant")
    );
}

#[test]
fn convergent_planner_stops_after_tool_call_sequence_completes() {
    let result = simulate_convergence(
        vec![
            LlmResponse::ToolCall("search".to_string(), json!({"pattern": "foo"})),
            LlmResponse::Text("Found it: foo is defined in bar.rs".to_string()),
        ],
        10,
    );

    assert!(result.converged, "should converge");
    assert_eq!(result.rounds_used, 2, "2 rounds (tool call + text)");
    assert!(
        result
            .final_text
            .as_ref()
            .map_or(false, |t| t.contains("Found it")),
        "final text should contain result"
    );
}

#[test]
fn convergent_planner_stops_after_five_tool_calls() {
    let mut responses: Vec<LlmResponse> = (0..5)
        .map(|_| LlmResponse::ToolCall("search".to_string(), json!({"pattern": "x"})))
        .collect();
    responses.push(LlmResponse::Text("Done after 5 calls".to_string()));

    let result = simulate_convergence(responses, 10);

    assert!(result.converged, "should converge");
    assert_eq!(result.rounds_used, 6, "6 rounds (5 tool + final text)");
    assert!(result.rounds_used < 10, "well under max_rounds");
}

#[test]
fn non_convergent_planner_hits_max_rounds() {
    let tool_only: Vec<LlmResponse> = (0..10)
        .map(|_| LlmResponse::ToolCall("search".to_string(), json!({"pattern": "foo"})))
        .collect();

    let result = simulate_convergence(tool_only, 10);

    assert!(!result.converged, "should NOT converge");
    assert_eq!(result.rounds_used, 10, "max_rounds reached");
    assert!(result.final_text.is_none(), "no final text");
}

#[test]
fn convergent_planner_respects_max_rounds_boundary() {
    // Exactly at max_rounds — tool calls for 9 rounds, text on 10th
    let mut responses: Vec<LlmResponse> = (0..9)
        .map(|_| LlmResponse::ToolCall("search".to_string(), json!({"pattern": "x"})))
        .collect();
    responses.push(LlmResponse::Text("Just in time".to_string()));

    let result = simulate_convergence(responses, 10);

    assert!(result.converged, "should converge exactly at boundary");
    assert_eq!(result.rounds_used, 10, "round 10 = max_rounds");
    assert_eq!(result.final_text.as_deref(), Some("Just in time"));
}

#[test]
#[should_panic(expected = "planner made 1 requests but only 0 responses were planned")]
fn empty_response_list_panics_as_test_guard() {
    // Zero planned responses exercises the test guard (not a realistic scenario).
    simulate_convergence(vec![], 10);
}

#[test]
fn single_tool_call_then_converge() {
    let result = simulate_convergence(
        vec![
            LlmResponse::ToolCall("filesystem_read".to_string(), json!({"path": "foo.rs"})),
            LlmResponse::Text("Here is the file content".to_string()),
        ],
        10,
    );

    assert!(result.converged);
    assert_eq!(result.rounds_used, 2);
}

#[test]
fn multiple_unique_tools_in_chain() {
    let result = simulate_convergence(
        vec![
            LlmResponse::ToolCall("search".to_string(), json!({"pattern": "class Foo"})),
            LlmResponse::ToolCall("filesystem_read".to_string(), json!({"path": "foo.rs"})),
            LlmResponse::ToolCall("directory_list".to_string(), json!({"path": "src"})),
            LlmResponse::Text("Here is the project structure".to_string()),
        ],
        10,
    );

    assert!(result.converged, "should converge after 4 rounds");
    assert_eq!(result.rounds_used, 4);
}
