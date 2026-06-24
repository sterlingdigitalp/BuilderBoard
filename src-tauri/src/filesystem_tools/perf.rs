use std::fmt::Display;
use std::time::Instant;

pub struct PerfSpan {
    label: &'static str,
    started: Instant,
}

impl PerfSpan {
    pub fn start(label: &'static str) -> Self {
        Self {
            label,
            started: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }

    pub fn finish(self) -> u128 {
        let elapsed = self.elapsed_ms();
        trace_perf_metric(self.label, elapsed);
        elapsed
    }
}

pub fn trace_perf_metric(label: &str, value: impl Display) {
    crate::runtime_diagnostics::trace_runtime_metric(label, value);
}