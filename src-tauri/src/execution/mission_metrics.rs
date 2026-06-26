use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MissionResult {
    Success,
    PartialSuccess,
    Failed,
}

impl MissionResult {
    pub fn as_label(&self) -> &'static str {
        match self {
            MissionResult::Success => "SUCCESS",
            MissionResult::PartialSuccess => "PARTIAL_SUCCESS",
            MissionResult::Failed => "FAILED",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MissionMetricsSummary {
    pub started_at: String,
    pub completed_at: String,
    pub planning_duration_ms: u128,
    pub first_tool_latency_ms: Option<u128>,
    pub tool_calls: u32,
    pub unique_tools: u32,
    pub tool_failures: u32,
    pub retries: u32,
    pub max_tool_chain_depth: u32,
    pub llm_generations: u32,
    pub llm_duration_ms: u128,
    pub tool_duration_ms: u128,
    pub total_duration_ms: u128,
    pub result: MissionResult,
    pub failure_category: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MissionMetrics {
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    start: Instant,
    planning_duration: Option<Duration>,
    first_tool_latency: Option<Duration>,
    tool_calls: u32,
    unique_tools: HashSet<String>,
    tool_failures: u32,
    retries: u32,
    max_tool_chain_depth: u32,
    llm_generations: u32,
    llm_duration: Duration,
    tool_duration: Duration,
    result: Option<MissionResult>,
    failure_category: Option<String>,
    failure_message: Option<String>,
}

impl MissionMetrics {
    pub fn start() -> Self {
        Self {
            started_at: Utc::now(),
            completed_at: None,
            start: Instant::now(),
            planning_duration: None,
            first_tool_latency: None,
            tool_calls: 0,
            unique_tools: HashSet::new(),
            tool_failures: 0,
            retries: 0,
            max_tool_chain_depth: 0,
            llm_generations: 0,
            llm_duration: Duration::ZERO,
            tool_duration: Duration::ZERO,
            result: None,
            failure_category: None,
            failure_message: None,
        }
    }

    pub fn record_planning_complete(&mut self) {
        if self.planning_duration.is_none() {
            self.planning_duration = Some(self.start.elapsed());
        }
    }

    pub fn record_llm_generation(&mut self, duration: Duration) {
        self.llm_generations += 1;
        self.llm_duration += duration;
    }

    pub fn record_tool_call_detected(&mut self, tool_name: &str, round: u32) {
        if self.first_tool_latency.is_none() {
            self.first_tool_latency = Some(self.start.elapsed());
        }
        self.tool_calls += 1;
        self.unique_tools.insert(tool_name.to_string());
        self.max_tool_chain_depth = self.max_tool_chain_depth.max(round);
    }

    pub fn record_tool_completion(&mut self, duration: Duration, success: bool) {
        self.tool_duration += duration;
        if !success {
            self.tool_failures += 1;
        }
    }

    pub fn record_tool_failure(&mut self) {
        self.tool_failures += 1;
    }

    pub fn record_retry(&mut self) {
        self.retries += 1;
    }

    pub fn complete_success(&mut self) {
        let result = if self.tool_failures > 0 {
            MissionResult::PartialSuccess
        } else {
            MissionResult::Success
        };
        self.complete(result, None, None);
    }

    pub fn complete_failed(&mut self, category: impl Into<String>, message: impl Into<String>) {
        self.complete(
            MissionResult::Failed,
            Some(category.into()),
            Some(message.into()),
        );
    }

    pub fn summary(&self) -> MissionMetricsSummary {
        let completed_at = self.completed_at.unwrap_or_else(Utc::now);
        MissionMetricsSummary {
            started_at: self.started_at.to_rfc3339(),
            completed_at: completed_at.to_rfc3339(),
            planning_duration_ms: millis(self.planning_duration.unwrap_or_default()),
            first_tool_latency_ms: self.first_tool_latency.map(millis),
            tool_calls: self.tool_calls,
            unique_tools: self.unique_tools.len() as u32,
            tool_failures: self.tool_failures,
            retries: self.retries,
            max_tool_chain_depth: self.max_tool_chain_depth,
            llm_generations: self.llm_generations,
            llm_duration_ms: millis(self.llm_duration),
            tool_duration_ms: millis(self.tool_duration),
            total_duration_ms: millis(self.start.elapsed()),
            result: self.result.clone().unwrap_or(MissionResult::Failed),
            failure_category: self.failure_category.clone(),
            failure_message: self.failure_message.clone(),
        }
    }

    pub fn render_block(&self) -> String {
        self.summary().render_block()
    }

    fn complete(
        &mut self,
        result: MissionResult,
        failure_category: Option<String>,
        failure_message: Option<String>,
    ) {
        if self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }
        self.result = Some(result);
        self.failure_category = failure_category;
        self.failure_message = failure_message;
    }
}

impl MissionMetricsSummary {
    pub fn render_block(&self) -> String {
        let mut lines = vec![
            "Mission Metrics".to_string(),
            format!(
                "Planning time:        {}",
                seconds(self.planning_duration_ms)
            ),
            format!(
                "First tool latency:   {}",
                self.first_tool_latency_ms
                    .map(seconds)
                    .unwrap_or_else(|| "N/A".to_string())
            ),
            format!("Tool calls:           {}", self.tool_calls),
            format!("Unique tools:         {}", self.unique_tools),
            format!("Tool failures:        {}", self.tool_failures),
            format!("Retries:              {}", self.retries),
            format!("Max tool depth:       {}", self.max_tool_chain_depth),
            format!("LLM generations:      {}", self.llm_generations),
            format!("LLM time:             {}", seconds(self.llm_duration_ms)),
            format!("Tool time:            {}", seconds(self.tool_duration_ms)),
            format!("Total duration:       {}", seconds(self.total_duration_ms)),
            format!("Result:               {}", self.result.as_label()),
        ];

        if let Some(message) = &self.failure_message {
            lines.push(format!("Failure Reason:       {}", message));
        }

        lines.join("\n")
    }
}

fn millis(duration: Duration) -> u128 {
    duration.as_millis()
}

fn seconds(ms: u128) -> String {
    format!("{:.2} s", ms as f64 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_successful_single_tool_metrics() {
        let mut metrics = MissionMetrics::start();
        metrics.record_planning_complete();
        metrics.record_llm_generation(Duration::from_millis(800));
        metrics.record_tool_call_detected("filesystem.write", 1);
        metrics.record_tool_completion(Duration::from_millis(40), true);
        metrics.record_llm_generation(Duration::from_millis(500));
        metrics.complete_success();

        let summary = metrics.summary();
        assert_eq!(summary.result, MissionResult::Success);
        assert_eq!(summary.tool_calls, 1);
        assert_eq!(summary.unique_tools, 1);
        assert_eq!(summary.tool_failures, 0);
        assert_eq!(summary.llm_generations, 2);
        assert!(metrics
            .render_block()
            .contains("Result:               SUCCESS"));
    }

    #[test]
    fn renders_partial_success_when_tool_fails_before_completion() {
        let mut metrics = MissionMetrics::start();
        metrics.record_tool_call_detected("filesystem.read", 1);
        metrics.record_tool_completion(Duration::from_millis(10), false);
        metrics.complete_success();

        let summary = metrics.summary();
        assert_eq!(summary.result, MissionResult::PartialSuccess);
        assert_eq!(summary.tool_failures, 1);
        assert!(metrics
            .render_block()
            .contains("Result:               PARTIAL_SUCCESS"));
    }

    #[test]
    fn renders_failed_metrics_with_reason() {
        let mut metrics = MissionMetrics::start();
        metrics.record_retry();
        metrics.complete_failed(
            "max_tool_rounds",
            "Maximum number of tool call rounds reached",
        );

        let block = metrics.render_block();
        assert!(block.contains("Result:               FAILED"));
        assert!(block.contains("Failure Reason:       Maximum number of tool call rounds reached"));
    }
}
