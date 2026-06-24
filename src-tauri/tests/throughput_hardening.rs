use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use builderboard_lib::filesystem_intent::route_filesystem_tools;
use builderboard_lib::stream_persistence::StreamPersistenceService;
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::models::CreatePaneRequest;
use builderboard_lib::storage::repositories::{
    messages::MessageRepository,
    panes::PaneRepository,
};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_database(name: &str) -> Database {
    let unique = TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir()
        .join("builderboard-tests")
        .join(format!("{name}-{unique}"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
    Database::initialize_at(path).expect("initialize database")
}

fn seed_stream_fixture(database: &Database) -> (String, String) {
    let root = std::env::temp_dir().join("builderboard-throughput-root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(root.join("package.json"), r#"{"name":"throughput"}"#).expect("write package");
    std::fs::create_dir_all(root.join("src")).expect("create src");

    let project = project_create_from_folder_with_database(
        database,
        &root.display().to_string(),
        Some(false),
    )
    .expect("create project");

    let pane_id = database
        .with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(project.id),
                    title: Some("Throughput pane".to_string()),
                    sort_order: None,
                },
            )?;
            Ok(pane.id)
        })
        .expect("create pane");

    (pane_id, String::new())
}

#[test]
fn security_review_bundle_is_bounded() {
    let routed = route_filesystem_tools("Run a security review of this project");
    assert!(
        routed.tools.len() <= 14,
        "security review bundle should stay bounded, got {}",
        routed.tools.len()
    );
    assert!(!routed.tools.is_empty());
}

#[test]
fn concurrent_readers_wait_while_stream_chunk_flush_holds_lock() {
    let database = Arc::new(temp_database("throughput-contention.db"));
    let (pane_id, _) = seed_stream_fixture(&database);

    let holder_db = Arc::clone(&database);
    let holder = thread::spawn(move || {
        holder_db
            .with_connection_labeled("throughput_hold_chunk_flush", |connection| {
                thread::sleep(Duration::from_millis(150));
                PaneRepository::get_open_for_execution(connection, &pane_id)?;
                Ok(())
            })
            .expect("hold chunk flush lock");
    });

    thread::sleep(Duration::from_millis(20));
    let probe_started = Instant::now();
    database
        .with_connection_labeled("throughput_probe_list", PaneRepository::list_shell_open)
        .expect("probe pane list");
    let probe_wait_ms = probe_started.elapsed().as_millis();

    holder.join().expect("holder thread");
    assert!(
        probe_wait_ms >= 120,
        "pane_list should wait behind prepare lock, got {probe_wait_ms}ms"
    );
}

#[test]
fn persistence_worker_preserves_append_order() {
    let database = Arc::new(temp_database("throughput-persist-order.db"));
    let service = StreamPersistenceService::new(Arc::clone(&database));
    let root = std::env::temp_dir().join("builderboard-throughput-persist-root");
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    fs::write(root.join("package.json"), r#"{"name":"persist"}"#).expect("write package");

    let project = project_create_from_folder_with_database(
        &database,
        &root.display().to_string(),
        Some(false),
    )
    .expect("create project");

    let assistant_message_id = database
        .with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(project.id),
                    title: Some("Persist pane".to_string()),
                    sort_order: None,
                },
            )?;
            let turn = MessageRepository::create_conversation_turn(
                connection,
                builderboard_lib::storage::models::MessageCreateRequest {
                    pane_id: pane.id,
                    content: "hello".to_string(),
                    content_type: Some("text".to_string()),
                    metadata_json: None,
                },
            )?;
            Ok(turn.assistant_message.id)
        })
        .expect("seed assistant message");

    service
        .enqueue_append("pane-1", &assistant_message_id, "abc")
        .expect("enqueue abc");
    service
        .enqueue_append("pane-1", &assistant_message_id, "def")
        .expect("enqueue def");
    service
        .drain_message_blocking(&assistant_message_id)
        .expect("drain");
    drop(service);

    let message = database
        .with_connection(|connection| MessageRepository::get_by_id(connection, &assistant_message_id))
        .expect("load message");
    assert_eq!(message.content, "abcdef");
}

#[test]
fn persistence_enqueue_returns_without_blocking_on_database_lock() {
    let database = Arc::new(temp_database("throughput-persist-queue.db"));
    let service = StreamPersistenceService::new(Arc::clone(&database));
    let holder_db = Arc::clone(&database);
    let lock_acquired = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let lock_acquired_for_holder = Arc::clone(&lock_acquired);
    let holder = thread::spawn(move || {
        holder_db
            .with_connection_labeled("throughput_persist_hold", |_connection| {
                lock_acquired_for_holder.store(true, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(200));
                Ok(())
            })
            .expect("hold lock");
    });

    let wait_started = Instant::now();
    while !lock_acquired.load(Ordering::SeqCst) {
        assert!(
            wait_started.elapsed() < Duration::from_secs(2),
            "holder failed to acquire database lock"
        );
        thread::sleep(Duration::from_millis(5));
    }

    let enqueue_started = Instant::now();
    service
        .enqueue_append("pane-1", "message-1", "delta")
        .expect("enqueue should not wait on database mutex");
    let enqueue_ms = enqueue_started.elapsed().as_millis();

    holder.join().expect("holder thread");
    assert!(
        enqueue_ms < 25,
        "enqueue should return immediately, took {enqueue_ms}ms"
    );
    drop(service);
}

#[test]
fn append_stream_delta_batches_multiple_chunks_in_one_write() {
    let database = temp_database("throughput-stream-append.db");
    let (pane_id, _) = seed_stream_fixture(&database);

    let assistant_message_id = database
        .with_connection(|connection| {
            let turn = MessageRepository::create_conversation_turn(
                connection,
                builderboard_lib::storage::models::MessageCreateRequest {
                    pane_id,
                    content: "hello".to_string(),
                    content_type: Some("text".to_string()),
                    metadata_json: None,
                },
            )?;
            Ok(turn.assistant_message.id)
        })
        .expect("create assistant placeholder");

    database
        .with_connection(|connection| {
            MessageRepository::append_stream_delta(connection, &assistant_message_id, "abc")?;
            MessageRepository::append_stream_delta(connection, &assistant_message_id, "def")?;
            let message = MessageRepository::get_by_id(connection, &assistant_message_id)?;
            assert_eq!(message.content, "abcdef");
            assert_eq!(message.status, "streaming");
            Ok(())
        })
        .expect("append stream deltas");
}