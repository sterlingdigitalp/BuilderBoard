use std::sync::mpsc::{self, RecvTimeoutError, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::storage::commands::flush_stream_delta;
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{MessageCompleteRequest, MessageErrorRequest};
use crate::storage::repositories::messages::MessageRepository;

const WORKER_FLUSH_BYTES: usize = 384;
const WORKER_POLL_MS: u64 = 25;

#[derive(Debug)]
enum PersistCommand {
    AppendDelta {
        message_id: String,
        delta: String,
    },
    MarkComplete {
        message_id: String,
    },
    MarkError {
        message_id: String,
        error_code: String,
        error_message: String,
    },
    Drain {
        message_id: String,
        ack: SyncSender<Result<(), StorageError>>,
    },
}

#[derive(Debug)]
struct PersistEnvelope {
    pane_id: String,
    command: PersistCommand,
}

pub struct StreamPersistenceService {
    sender: Option<Sender<PersistEnvelope>>,
    worker: Option<JoinHandle<()>>,
}

impl StreamPersistenceService {
    pub fn new(database: Arc<Database>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("stream-persistence-worker".into())
            .spawn(move || run_persistence_worker(database, receiver))
            .expect("spawn stream persistence worker");

        Self {
            sender: Some(sender),
            worker: Some(worker),
        }
    }

    pub fn enqueue_append(
        &self,
        pane_id: &str,
        message_id: &str,
        delta: impl Into<String>,
    ) -> Result<(), StorageError> {
        self.send(PersistEnvelope {
            pane_id: pane_id.to_string(),
            command: PersistCommand::AppendDelta {
                message_id: message_id.to_string(),
                delta: delta.into(),
            },
        })
    }

    pub fn enqueue_complete(&self, pane_id: &str, message_id: &str) -> Result<(), StorageError> {
        self.send(PersistEnvelope {
            pane_id: pane_id.to_string(),
            command: PersistCommand::MarkComplete {
                message_id: message_id.to_string(),
            },
        })
    }

    pub fn enqueue_error(
        &self,
        pane_id: &str,
        message_id: &str,
        error_code: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Result<(), StorageError> {
        self.send(PersistEnvelope {
            pane_id: pane_id.to_string(),
            command: PersistCommand::MarkError {
                message_id: message_id.to_string(),
                error_code: error_code.into(),
                error_message: error_message.into(),
            },
        })
    }

    pub fn drain_message_blocking(&self, message_id: &str) -> Result<(), StorageError> {
        let (ack, receiver) = mpsc::sync_channel(1);
        self.send(PersistEnvelope {
            pane_id: String::new(),
            command: PersistCommand::Drain {
                message_id: message_id.to_string(),
                ack,
            },
        })?;
        receiver
            .recv()
            .map_err(|_| StorageError::InvalidInput("persistence drain cancelled".to_string()))?
    }

    pub async fn drain_message(&self, message_id: &str) -> Result<(), StorageError> {
        let message_id = message_id.to_string();
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| {
                StorageError::InvalidInput("stream persistence worker unavailable".to_string())
            })?
            .clone();
        tauri::async_runtime::spawn_blocking(move || {
            let (ack, receiver) = mpsc::sync_channel(1);
            sender
                .send(PersistEnvelope {
                    pane_id: String::new(),
                    command: PersistCommand::Drain { message_id, ack },
                })
                .map_err(|_| {
                    StorageError::InvalidInput("stream persistence worker unavailable".to_string())
                })?;
            receiver.recv().map_err(|_| {
                StorageError::InvalidInput("persistence drain cancelled".to_string())
            })?
        })
        .await
        .map_err(|_| StorageError::InvalidInput("persistence drain task cancelled".to_string()))?
    }

    fn send(&self, envelope: PersistEnvelope) -> Result<(), StorageError> {
        self.sender
            .as_ref()
            .ok_or_else(|| {
                StorageError::InvalidInput("stream persistence worker unavailable".to_string())
            })?
            .send(envelope)
            .map_err(|_| {
                StorageError::InvalidInput("stream persistence worker unavailable".to_string())
            })
    }
}

impl Drop for StreamPersistenceService {
    fn drop(&mut self) {
        // Drop the sender first so the worker observes Disconnected and exits.
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_persistence_worker(database: Arc<Database>, receiver: mpsc::Receiver<PersistEnvelope>) {
    let mut pending: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    loop {
        match receiver.recv_timeout(Duration::from_millis(WORKER_POLL_MS)) {
            Ok(envelope) => {
                if let Err(error) = handle_envelope(&database, &mut pending, envelope) {
                    crate::runtime_diagnostics::trace_runtime_phase(
                        "stream_persistence_worker_error",
                        error,
                    );
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                for message_id in pending.keys().cloned().collect::<Vec<_>>() {
                    if should_flush_pending(pending.get(&message_id)) {
                        let _ = flush_pending_for_message(&database, &mut pending, &message_id);
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                for message_id in pending.keys().cloned().collect::<Vec<_>>() {
                    let _ = flush_pending_for_message(&database, &mut pending, &message_id);
                }
                break;
            }
        }
    }
}

fn handle_envelope(
    database: &Database,
    pending: &mut std::collections::HashMap<String, String>,
    envelope: PersistEnvelope,
) -> Result<(), StorageError> {
    match envelope.command {
        PersistCommand::AppendDelta { message_id, delta } => {
            if !delta.is_empty() {
                pending
                    .entry(message_id.clone())
                    .or_default()
                    .push_str(&delta);
            }
            if should_flush_pending(pending.get(&message_id)) {
                flush_pending_for_message(database, pending, &message_id)?;
            }
        }
        PersistCommand::MarkComplete { message_id } => {
            flush_pending_for_message(database, pending, &message_id)?;
            database.with_connection_labeled("stream_persist_complete", |connection| {
                let latest = MessageRepository::get_by_id(connection, &message_id)?;
                if latest.status != "complete" {
                    MessageRepository::mark_complete(
                        connection,
                        MessageCompleteRequest {
                            message_id: message_id.clone(),
                            content: None,
                            token_count_input: None,
                            token_count_output: None,
                            metadata_json: None,
                        },
                    )?;
                }
                Ok(())
            })?;
        }
        PersistCommand::MarkError {
            message_id,
            error_code,
            error_message,
        } => {
            flush_pending_for_message(database, pending, &message_id)?;
            database.with_connection_labeled("stream_persist_error", |connection| {
                MessageRepository::mark_error(
                    connection,
                    MessageErrorRequest {
                        message_id,
                        error_code,
                        error_message,
                    },
                )
            })?;
        }
        PersistCommand::Drain { message_id, ack } => {
            let result = flush_pending_for_message(database, pending, &message_id);
            let _ = ack.send(result);
        }
    }

    Ok(())
}

fn should_flush_pending(pending: Option<&String>) -> bool {
    pending.is_some_and(|value| value.len() >= WORKER_FLUSH_BYTES)
}

fn flush_pending_for_message(
    database: &Database,
    pending: &mut std::collections::HashMap<String, String>,
    message_id: &str,
) -> Result<(), StorageError> {
    let Some(delta) = pending.remove(message_id) else {
        return Ok(());
    };
    if delta.is_empty() {
        return Ok(());
    }

    database.with_connection_labeled("stream_persist_append", |connection| {
        flush_stream_delta(connection, message_id, &delta)
    })
}
