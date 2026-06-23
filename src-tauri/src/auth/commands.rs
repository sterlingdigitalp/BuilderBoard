use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::auth::oauth_service::{oauth_log, OAuthService};
use crate::auth::CredentialService;
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{OAuthCompleteEvent, OAuthErrorEvent, OAuthStartResult};
use crate::storage::repositories::accounts::AccountRepository;

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
    let app_success = app.clone();
    let app_error = app;
    let database_success = database.clone();
    oauth.oauth_start(
        database,
        credentials,
        provider_id,
        open_browser,
        move |event| emit_oauth_success(&app_success, database_success.clone(), event),
        move |event| emit_oauth_error(&app_error, event),
    )
}

fn emit_oauth_success(app: &AppHandle, database: Arc<Database>, event: OAuthCompleteEvent) {
    let app_for_thread = app.clone();
    let app_for_emit = app.clone();
    let account_id = event.account_id.clone();

    if let Err(err) = app_for_thread.run_on_main_thread(move || {
        match app_for_emit.emit("oauth_complete", &event) {
            Ok(()) => oauth_log(format!("Emitted oauth_complete for account {account_id}")),
            Err(error) => oauth_log(format!("Failed to emit oauth_complete: {error}")),
        }

        match database.with_connection(|connection| AccountRepository::get_by_id(connection, &account_id))
        {
            Ok(account) => match app_for_emit.emit("account_created", &account) {
                Ok(()) => oauth_log(format!("Emitted account_created for account {account_id}")),
                Err(error) => oauth_log(format!("Failed to emit account_created: {error}")),
            },
            Err(error) => oauth_log(format!("Failed to load account for account_created event: {error}")),
        }
    }) {
        oauth_log(format!("Failed to schedule oauth success events on main thread: {err}"));
    }
}

fn emit_oauth_error(app: &AppHandle, event: OAuthErrorEvent) {
    let app_for_thread = app.clone();
    let app_for_emit = app.clone();
    let provider_id = event.provider_id.clone();

    if let Err(err) = app_for_thread.run_on_main_thread(move || {
        match app_for_emit.emit("oauth_error", &event) {
            Ok(()) => oauth_log(format!("Emitted oauth_error for provider {provider_id}")),
            Err(error) => oauth_log(format!("Failed to emit oauth_error: {error}")),
        }
    }) {
        oauth_log(format!("Failed to schedule oauth_error event on main thread: {err}"));
    }
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