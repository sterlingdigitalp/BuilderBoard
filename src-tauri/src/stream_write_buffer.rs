use std::sync::Arc;

use tauri::{AppHandle, Runtime};

use crate::storage::commands::emit_stream_chunk;
use crate::storage::error::StorageError;
use crate::stream_persistence::StreamPersistenceService;

pub struct StreamWriteBuffer {
    persistence: Arc<StreamPersistenceService>,
    pane_id: String,
    message_id: String,
}

impl StreamWriteBuffer {
    pub fn new(
        persistence: Arc<StreamPersistenceService>,
        pane_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> Self {
        Self {
            persistence,
            pane_id: pane_id.into(),
            message_id: message_id.into(),
        }
    }

    pub fn push<R: Runtime>(&self, app: &AppHandle<R>, delta: &str) -> Result<(), StorageError> {
        if delta.is_empty() {
            return Ok(());
        }

        emit_stream_chunk(app, &self.pane_id, &self.message_id, delta);
        self.persistence
            .enqueue_append(&self.pane_id, &self.message_id, delta.to_string())
    }

    pub async fn finish(&self) -> Result<(), StorageError> {
        self.persistence.drain_message(&self.message_id).await
    }

    pub fn enqueue_complete(&self) -> Result<(), StorageError> {
        self.persistence
            .enqueue_complete(&self.pane_id, &self.message_id)
    }

    pub async fn finish_with_complete<R: Runtime>(
        &self,
        app: &AppHandle<R>,
    ) -> Result<(), StorageError> {
        self.enqueue_complete()?;
        self.finish().await?;
        crate::storage::commands::emit_stream_complete(app, &self.pane_id, &self.message_id);
        Ok(())
    }
}
