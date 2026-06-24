use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::repositories::panes::PaneRepository;

fn temp_database(name: &str) -> Database {
    let path = std::env::temp_dir()
        .join("builderboard-tests")
        .join(name);
    let _ = std::fs::remove_file(&path);
    Database::initialize_at(path).expect("initialize database")
}

#[test]
fn database_mutex_serializes_concurrent_readers() {
    let database = Arc::new(temp_database("runtime-blocking-mutex.db"));
    let holder_db = Arc::clone(&database);
    let holder = thread::spawn(move || {
        holder_db
            .with_connection_labeled("diagnostic_hold", |_connection| {
                thread::sleep(Duration::from_millis(250));
                Ok(())
            })
            .expect("hold connection");
    });

    thread::sleep(Duration::from_millis(20));
    let probe_started = Instant::now();
    database
        .with_connection_labeled("diagnostic_probe", PaneRepository::list_shell_open)
        .expect("probe pane list");
    let probe_wait_ms = probe_started.elapsed().as_millis();

    holder.join().expect("holder thread");
    assert!(
        probe_wait_ms >= 200,
        "expected pane_list probe to wait on mutex, got {probe_wait_ms}ms"
    );
}

#[test]
fn fire_and_forget_spawn_returns_without_waiting_for_background_work() {
    let started = Instant::now();
    let handle = thread::spawn(|| {
        thread::sleep(Duration::from_millis(120));
        "scan-complete"
    });
    let spawn_return_ms = started.elapsed().as_millis();
    let _ = handle.join().expect("join enrichment thread");

    assert!(
        spawn_return_ms < 25,
        "fire-and-forget spawn should return immediately, got {spawn_return_ms}ms"
    );
}