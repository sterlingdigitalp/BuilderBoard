use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static COMMAND_THREAD_BLOCK_MS: AtomicU64 = AtomicU64::new(0);

pub fn runtime_trace_enabled() -> bool {
    std::env::var("BUILDERBOARD_TRACE_RUNTIME").as_deref() == Ok("1")
        || std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() == Ok("1")
}

pub fn trace_runtime_metric(label: &str, value: impl Display) {
    if runtime_trace_enabled() {
        println!("{label}={value}");
    }
}

pub fn trace_runtime_phase(phase: &str, detail: impl Display) {
    if runtime_trace_enabled() {
        println!("RUNTIME_PHASE={phase} detail={detail}");
    }
}

pub struct RuntimeSpan {
    label: &'static str,
    started: Instant,
    accumulate_command_block: bool,
}

impl RuntimeSpan {
    pub fn start(label: &'static str) -> Self {
        Self {
            label,
            started: Instant::now(),
            accumulate_command_block: false,
        }
    }

    pub fn start_command_block(label: &'static str) -> Self {
        Self {
            label,
            started: Instant::now(),
            accumulate_command_block: true,
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }

    pub fn finish(self) -> u128 {
        let elapsed = self.elapsed_ms();
        trace_runtime_metric(self.label, elapsed);
        if self.accumulate_command_block {
            COMMAND_THREAD_BLOCK_MS.fetch_add(elapsed as u64, Ordering::Relaxed);
        }
        elapsed
    }
}

pub fn record_command_thread_block_ms(duration_ms: u128) {
    COMMAND_THREAD_BLOCK_MS.fetch_add(duration_ms as u64, Ordering::Relaxed);
}

pub fn emit_main_thread_block_total() {
    let total = COMMAND_THREAD_BLOCK_MS.load(Ordering::Relaxed);
    trace_runtime_metric("MAIN_THREAD_BLOCK_MS", total);
}

pub struct DatabaseLockSpan {
    operation: &'static str,
    wait_started: Instant,
    hold_started: Option<Instant>,
}

impl DatabaseLockSpan {
    pub fn waiting(operation: &'static str) -> Self {
        Self {
            operation,
            wait_started: Instant::now(),
            hold_started: None,
        }
    }

    pub fn acquired(&mut self) {
        let wait_ms = self.wait_started.elapsed().as_millis();
        if wait_ms > 0 {
            trace_runtime_metric(
                &format!("DB_LOCK_WAIT_MS_{}", self.operation),
                wait_ms,
            );
            trace_runtime_metric("DB_LOCK_WAIT_MS", wait_ms);
            if wait_ms >= 5 {
                record_command_thread_block_ms(wait_ms);
            }
        }
        self.hold_started = Some(Instant::now());
    }

    pub fn finish(self) {
        if let Some(hold_started) = self.hold_started {
            let hold_ms = hold_started.elapsed().as_millis();
            trace_runtime_metric(
                &format!("DB_LOCK_HOLD_MS_{}", self.operation),
                hold_ms,
            );
            trace_runtime_metric("DB_LOCK_HOLD_MS", hold_ms);
        }
    }
}