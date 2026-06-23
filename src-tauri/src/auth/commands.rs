use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::auth::oauth_service::OAuthService;
use crate::auth::CredentialService;
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::OAuthStartResult;

#[cfg(test)]
use crate::storage::models::{OAuthCompleteEvent, OAuthErrorEvent};

#[tauri::command]
pub fn oauth_start(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    credentials: State<'_, Arc<CredentialService>>,
    oauth: State<'_, Arc<OAuthService>>,
    provider_id: String,
) -> Result<OAuthStartResult, String> {
    oauth_start_with_service(
        app,
        database.inner().clone(),
        credentials.inner().clone(),
        oauth.inner().as_ref(),
        &provider_id,
        true,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn oauth_cancel(
    oauth: State<'_, Arc<OAuthService>>,
    provider_id: String,
) -> Result<(), String> {
    oauth
        .oauth_cancel(&provider_id)
        .map_err(|error| error.to_string())
}

pub fn oauth_start_with_service(
    app: AppHandle,
    database: Arc<Database>,
    credentials: Arc<CredentialService>,
    oauth: &OAuthService,
    provider_id: &str,
    open_browser: bool,
) -> Result<OAuthStartResult, StorageError> {
    let app_complete = app.clone();
    let app_error = app;
    oauth.oauth_start(
        database,
        credentials,
        provider_id,
        open_browser,
        move |event| {
            let _ = app_complete.emit("oauth_complete", event);
        },
        move |event| {
            let _ = app_error.emit("oauth_error", event);
        },
    )
}

#[cfg(test)]
pub fn oauth_start_for_test<H, B>(
    database: Arc<Database>,
    credentials: Arc<CredentialService>,
    oauth: &OAuthService<H, B>,
    provider_id: &str,
    open_browser: bool,
    on_complete: impl Fn(OAuthCompleteEvent) + Send + 'static,
    on_error: impl Fn(OAuthErrorEvent) + Send + 'static,
) -> Result<OAuthStartResult, StorageError>
where
    H: crate::auth::oauth_service::OAuthHttpClient + Clone + Send + Sync + 'static,
    B: crate::auth::oauth_service::SystemBrowser + Clone + Send + Sync + 'static,
{
    oauth.oauth_start(
        database,
        credentials,
        provider_id,
        open_browser,
        on_complete,
        on_error,
    )
}