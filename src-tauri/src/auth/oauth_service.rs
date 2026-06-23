use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::credential_service::CredentialService;
use crate::storage::db::Database;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{AccountDto, OAuthCompleteEvent, OAuthErrorEvent, OAuthStartResult};
use crate::storage::repositories::accounts::AccountRepository;
use crate::storage::repositories::providers::{OAuthProviderConfig, ProviderRepository};

pub const OAUTH_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
pub const OAUTH_SUPPORTED_PROVIDERS: &[&str] = &["google"];
pub const GOOGLE_CLIENT_ID_ENV: &str = "BUILDERBOARD_GOOGLE_CLIENT_ID";
pub const GOOGLE_CLIENT_SECRET_ENV: &str = "BUILDERBOARD_GOOGLE_CLIENT_SECRET";

#[derive(Debug, Clone)]
pub struct GoogleOAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub(crate) fn oauth_log(message: impl AsRef<str>) {
    eprintln!("[OAuth] {}", message.as_ref());
}

fn oauth_log_response(stage: &str, status: u16, body: &str) {
    oauth_log(format!(
        "{stage}: {status} {}",
        truncate_for_log(body, 240)
    ));
}

fn truncate_for_log(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}…", &value[..max_len])
    }
}

fn mask_credential(value: &str) -> String {
    if value.len() <= 8 {
        "***".to_string()
    } else {
        format!("{}…{}", &value[..4], &value[value.len() - 4..])
    }
}

fn log_token_exchange_request(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    grant_type: &str,
) {
    oauth_log(format!("Token exchange endpoint: {token_url}"));
    oauth_log("Token exchange Content-Type: application/x-www-form-urlencoded");
    oauth_log(format!(
        "Token exchange body fields: grant_type={grant_type}, client_id={} (present, len={}), client_secret={} (len={}), code=<redacted>, code_verifier=<redacted>, redirect_uri={redirect_uri}",
        mask_credential(client_id),
        client_id.len(),
        if client_secret.is_empty() {
            "MISSING".to_string()
        } else {
            format!("present ({})", mask_credential(client_secret))
        },
        client_secret.len(),
    ));
}

fn log_refresh_token_request(token_url: &str, client_id: &str, client_secret: &str) {
    oauth_log(format!("Token refresh endpoint: {token_url}"));
    oauth_log("Token refresh Content-Type: application/x-www-form-urlencoded");
    oauth_log(format!(
        "Token refresh body fields: grant_type=refresh_token, client_id={} (present, len={}), client_secret={} (len={}), refresh_token=<redacted>",
        mask_credential(client_id),
        client_id.len(),
        if client_secret.is_empty() {
            "MISSING".to_string()
        } else {
            format!("present ({})", mask_credential(client_secret))
        },
        client_secret.len(),
    ));
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfoResponse {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

pub trait OAuthHttpClient: Send + Sync {
    fn exchange_code(
        &self,
        token_url: &str,
        client_id: &str,
        client_secret: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> StorageResult<TokenResponse>;

    fn refresh_token(
        &self,
        token_url: &str,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> StorageResult<TokenResponse>;

    fn fetch_userinfo(&self, userinfo_url: &str, access_token: &str) -> StorageResult<UserInfoResponse>;
}

pub trait SystemBrowser: Send + Sync {
    fn open_url(&self, url: &str) -> StorageResult<()>;
}

#[derive(Clone)]
pub struct ReqwestOAuthClient;

impl OAuthHttpClient for ReqwestOAuthClient {
    fn exchange_code(
        &self,
        token_url: &str,
        client_id: &str,
        client_secret: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> StorageResult<TokenResponse> {
        log_token_exchange_request(
            token_url,
            client_id,
            client_secret,
            redirect_uri,
            "authorization_code",
        );

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("code", code),
                ("code_verifier", code_verifier),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .map_err(|err| {
                oauth_log(format!("Token exchange request failed: {err}"));
                StorageError::InvalidInput(format!("token exchange request failed: {err}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            oauth_log_response("Token exchange response", status.as_u16(), &body);
            return Err(StorageError::InvalidInput(format!(
                "token exchange failed: {} {}",
                status.as_u16(),
                body
            )));
        }

        oauth_log(format!("Token exchange response: {}", status.as_u16()));
        response
            .json::<TokenResponse>()
            .map_err(|err| {
                oauth_log(format!("Token exchange parse failed: {err}"));
                StorageError::InvalidInput(format!("invalid token response: {err}"))
            })
    }

    fn refresh_token(
        &self,
        token_url: &str,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> StorageResult<TokenResponse> {
        log_refresh_token_request(token_url, client_id, client_secret);

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
            ])
            .send()
            .map_err(|err| {
                oauth_log(format!("Token refresh request failed: {err}"));
                StorageError::InvalidInput(format!("token refresh request failed: {err}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            oauth_log_response("Token refresh response", status.as_u16(), &body);
            return Err(StorageError::InvalidInput(format!(
                "token refresh failed: {} {}",
                status.as_u16(),
                body
            )));
        }

        oauth_log(format!("Token refresh response: {}", status.as_u16()));

        response
            .json::<TokenResponse>()
            .map_err(|err| StorageError::InvalidInput(format!("invalid refresh response: {err}")))
    }

    fn fetch_userinfo(&self, userinfo_url: &str, access_token: &str) -> StorageResult<UserInfoResponse> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(userinfo_url)
            .bearer_auth(access_token)
            .send()
            .map_err(|err| {
                oauth_log(format!("Userinfo request failed: {err}"));
                StorageError::InvalidInput(format!("userinfo request failed: {err}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            oauth_log_response("Userinfo response", status.as_u16(), &body);
            return Err(StorageError::InvalidInput(format!(
                "userinfo request failed: {} {}",
                status.as_u16(),
                body
            )));
        }

        oauth_log(format!("Userinfo response: {}", status.as_u16()));
        response
            .json::<UserInfoResponse>()
            .map_err(|err| {
                oauth_log(format!("Userinfo parse failed: {err}"));
                StorageError::InvalidInput(format!("invalid userinfo response: {err}"))
            })
    }
}

#[derive(Clone)]
pub struct MacSystemBrowser;

impl SystemBrowser for MacSystemBrowser {
    fn open_url(&self, url: &str) -> StorageResult<()> {
        std::process::Command::new("open")
            .arg(url)
            .status()
            .map_err(StorageError::from)?;

        Ok(())
    }
}

#[derive(Clone)]
struct PendingOAuthSession {
    state: String,
    code_verifier: String,
    redirect_uri: String,
    oauth_config: OAuthProviderConfig,
    client_id: String,
    client_secret: String,
    cancel_flag: Arc<AtomicBool>,
}

pub struct OAuthService<H: OAuthHttpClient = ReqwestOAuthClient, B: SystemBrowser = MacSystemBrowser> {
    http: H,
    browser: B,
    google_credentials_resolver:
        Box<dyn Fn(&str) -> StorageResult<GoogleOAuthCredentials> + Send + Sync>,
    pending: Arc<Mutex<HashMap<String, PendingOAuthSession>>>,
}

impl OAuthService {
    pub fn production() -> OAuthService<ReqwestOAuthClient, MacSystemBrowser> {
        OAuthService {
            http: ReqwestOAuthClient,
            browser: MacSystemBrowser,
            google_credentials_resolver: Box::new(resolve_google_credentials),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<H, B> OAuthService<H, B>
where
    H: OAuthHttpClient + Clone + Send + Sync + 'static,
    B: SystemBrowser + Clone + Send + Sync + 'static,
{
    pub fn with_dependencies(
        http: H,
        browser: B,
        google_credentials_resolver: Box<
            dyn Fn(&str) -> StorageResult<GoogleOAuthCredentials> + Send + Sync,
        >,
    ) -> Self {
        Self {
            http,
            browser,
            google_credentials_resolver,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn oauth_start(
        &self,
        database: Arc<Database>,
        credentials: Arc<CredentialService>,
        provider_id: &str,
        open_browser: bool,
        on_complete: impl Fn(OAuthCompleteEvent) + Send + 'static,
        on_error: impl Fn(OAuthErrorEvent) + Send + 'static,
    ) -> StorageResult<OAuthStartResult> {
        Self::validate_oauth_provider(&database, provider_id)?;

        let oauth_config = database.with_connection(|connection| {
            ProviderRepository::get_oauth_config(connection, provider_id)
        })?;
        let google_credentials = (self.google_credentials_resolver)(provider_id)?;

        self.cancel_pending(provider_id);

        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier)?;
        let state = generate_state();
        let listener = bind_loopback_listener()?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}/callback");
        oauth_log(format!("Flow started for provider {provider_id}"));
        oauth_log(format!("Loopback redirect_uri: {redirect_uri}"));

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let session = PendingOAuthSession {
            state: state.clone(),
            code_verifier,
            redirect_uri: redirect_uri.clone(),
            oauth_config: oauth_config.clone(),
            client_id: google_credentials.client_id.clone(),
            client_secret: google_credentials.client_secret.clone(),
            cancel_flag: Arc::clone(&cancel_flag),
        };

        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| StorageError::InvalidInput("oauth session lock poisoned".to_string()))?;
            pending.insert(provider_id.to_string(), session);
        }

        let auth_url = build_authorization_url(
            &oauth_config.authorization_url,
            &google_credentials.client_id,
            &redirect_uri,
            &oauth_config.scopes,
            &state,
            &code_challenge,
        )?;

        if open_browser {
            self.browser.open_url(&auth_url)?;
            oauth_log("System browser launched");
        }

        oauth_log("Waiting for loopback callback");

        let service = Arc::new(OAuthServiceRunner {
            http: self.http.clone(),
            pending: Arc::clone(&self.pending),
        });

        let provider_id_owned = provider_id.to_string();
        thread::spawn(move || {
            let result = handle_loopback_callback(listener, &cancel_flag, |callback| {
                service.complete_callback(
                    &database,
                    &credentials,
                    &provider_id_owned,
                    callback,
                )
            });

            match result {
                Ok(account) => {
                    oauth_log(format!(
                        "Flow completed successfully: account_id={} label={}",
                        account.id, account.label
                    ));
                    on_complete(OAuthCompleteEvent {
                        account_id: account.id.clone(),
                        provider_id: account.provider_id.clone(),
                        label: account.label.clone(),
                    })
                }
                Err(error) => {
                    oauth_log(format!("Flow failed: {error}"));
                    on_error(map_oauth_error(&provider_id_owned, error))
                }
            }
        });

        Ok(OAuthStartResult { auth_url })
    }

    pub fn oauth_cancel(&self, provider_id: &str) -> StorageResult<()> {
        self.cancel_pending(provider_id);
        Ok(())
    }

    pub fn refresh_oauth_access_token(
        &self,
        database: &Database,
        credentials: &CredentialService,
        account_id: &str,
    ) -> StorageResult<String> {
        let (provider_id, credential_ref, label) = database.with_connection(|connection| {
            let account = AccountRepository::get_by_id(connection, account_id)?;
            if account.auth_type != "oauth" {
                return Err(StorageError::InvalidInput(
                    "account is not an oauth account".to_string(),
                ));
            }
            let credential_ref = AccountRepository::credential_ref(connection, account_id)?;
            Ok((account.provider_id, credential_ref, account.label))
        })?;

        let oauth_config = database.with_connection(|connection| {
            ProviderRepository::get_oauth_config(connection, &provider_id)
        })?;
        let google_credentials = (self.google_credentials_resolver)(&provider_id)?;
        let credential = credentials.read_oauth_credential(&credential_ref)?;

        if !CredentialService::oauth_access_token_needs_refresh(&credential)? {
            return Ok(credential.access_token);
        }

        let refreshed = self.http.refresh_token(
            &oauth_config.token_url,
            &google_credentials.client_id,
            &google_credentials.client_secret,
            &credential.refresh_token,
        )?;

        let updated = CredentialService::oauth_credential_from_token_response(
            refreshed.access_token,
            refreshed.refresh_token,
            refreshed.token_type,
            refreshed.expires_in,
            Some(&credential.refresh_token),
        )?;

        credentials.store_oauth_credential(
            &credential_ref,
            &label,
            &provider_id,
            &updated,
        )?;

        database.with_connection(|connection| {
            AccountRepository::update_oauth_token_metadata(
                connection,
                account_id,
                &updated.expires_at,
                refreshed.scope.as_deref(),
            )
        })?;

        Ok(updated.access_token)
    }

    fn cancel_pending(&self, provider_id: &str) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(session) = pending.remove(provider_id) {
                oauth_log(format!("Cancelling pending flow for provider {provider_id}"));
                session.cancel_flag.store(true, Ordering::SeqCst);
            }
        }
    }

    fn validate_oauth_provider(database: &Database, provider_id: &str) -> StorageResult<()> {
        if !OAUTH_SUPPORTED_PROVIDERS.contains(&provider_id) {
            return Err(StorageError::InvalidInput(format!(
                "provider {provider_id} does not support OAuth in Phase 3B"
            )));
        }

        database.with_connection(|connection| {
            let provider = ProviderRepository::get_enabled_by_id(connection, provider_id)?;
            if provider.auth_mode != "oauth" {
                return Err(StorageError::InvalidInput(format!(
                    "provider {provider_id} is not configured for oauth"
                )));
            }
            Ok(())
        })
    }
}

struct OAuthServiceRunner<H: OAuthHttpClient + Clone> {
    http: H,
    pending: Arc<Mutex<HashMap<String, PendingOAuthSession>>>,
}

impl<H> OAuthServiceRunner<H>
where
    H: OAuthHttpClient + Clone,
{
    fn complete_callback(
        &self,
        database: &Database,
        credentials: &CredentialService,
        provider_id: &str,
        callback: OAuthCallback,
    ) -> StorageResult<AccountDto> {
        oauth_log("Callback received");

        let session = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| StorageError::InvalidInput("oauth session lock poisoned".to_string()))?;
            pending.remove(provider_id).ok_or_else(|| {
                StorageError::InvalidInput("oauth session not found".to_string())
            })
        };

        let session = match session {
            Ok(session) => session,
            Err(error) => {
                oauth_log(format!("Pending session lookup failed: {error}"));
                return Err(error);
            }
        };

        if callback.state != session.state {
            oauth_log("State mismatch");
            return Err(StorageError::InvalidInput("oauth state mismatch".to_string()));
        }
        oauth_log("State validated");
        oauth_log(format!(
            "Exchanging token with redirect_uri: {}",
            session.redirect_uri
        ));

        let token_response = match self.http.exchange_code(
            &session.oauth_config.token_url,
            &session.client_id,
            &session.client_secret,
            &callback.code,
            &session.code_verifier,
            &session.redirect_uri,
        ) {
            Ok(response) => {
                oauth_log("Token exchange succeeded");
                response
            }
            Err(error) => {
                oauth_log(format!("Token exchange failed: {error}"));
                return Err(error);
            }
        };

        let oauth_credential = match CredentialService::oauth_credential_from_token_response(
            token_response.access_token.clone(),
            token_response.refresh_token,
            token_response.token_type,
            token_response.expires_in,
            None,
        ) {
            Ok(credential) => credential,
            Err(error) => {
                oauth_log(format!("Credential payload build failed: {error}"));
                return Err(error);
            }
        };

        let userinfo = match self.http.fetch_userinfo(
            &session.oauth_config.userinfo_url,
            &oauth_credential.access_token,
        ) {
            Ok(info) => {
                oauth_log("Userinfo request succeeded");
                info
            }
            Err(error) => {
                oauth_log(format!("Userinfo request failed: {error}"));
                return Err(error);
            }
        };

        let label = userinfo
            .email
            .clone()
            .or(userinfo.name.clone())
            .unwrap_or_else(|| "Google Account".to_string());

        let credential_ref = CredentialService::generate_credential_ref();
        if let Err(error) = credentials.store_oauth_credential(
            &credential_ref,
            &label,
            provider_id,
            &oauth_credential,
        ) {
            oauth_log(format!("Keychain storage failed: {error}"));
            return Err(error);
        }
        oauth_log("Keychain entry created");

        match database.with_connection(|connection| {
            AccountRepository::create_oauth_account(
                connection,
                provider_id,
                &label,
                &credential_ref,
                &userinfo.sub,
                userinfo.email.as_deref(),
                &oauth_credential.expires_at,
                token_response.scope.as_deref(),
                false,
            )
        }) {
            Ok(account) => {
                oauth_log(format!("Account creation succeeded: {}", account.id));
                Ok(account)
            }
            Err(error) => {
                oauth_log(format!("Account creation failed: {error}"));
                let _ = credentials.delete_credential(&credential_ref);
                Err(error)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct OAuthCallback {
    code: String,
    state: String,
}

fn handle_loopback_callback<F>(
    listener: TcpListener,
    cancel_flag: &AtomicBool,
    complete: F,
) -> StorageResult<AccountDto>
where
    F: FnOnce(OAuthCallback) -> StorageResult<AccountDto>,
{
    listener
        .set_nonblocking(true)
        .map_err(StorageError::from)?;

    let deadline = std::time::Instant::now() + OAUTH_CALLBACK_TIMEOUT;

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            oauth_log("Flow cancelled");
            return Err(StorageError::InvalidInput("oauth flow cancelled".to_string()));
        }

        if std::time::Instant::now() >= deadline {
            oauth_log("Flow timed out waiting for callback");
            return Err(StorageError::InvalidInput("oauth flow timed out".to_string()));
        }

        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    oauth_log(format!("Rejected non-loopback connection from {}", peer.ip()));
                    continue;
                }

                oauth_log(format!("Loopback connection accepted from {}", peer));
                let callback = match read_oauth_callback(&mut stream) {
                    Ok(callback) => callback,
                    Err(error) => {
                        oauth_log(format!("Callback parse failed: {error}"));
                        let _ = stream.write_all(error_response().as_bytes());
                        let _ = stream.flush();
                        return Err(error);
                    }
                };
                let result = complete(callback);
                let response_body = match &result {
                    Ok(_) => success_response(),
                    Err(_) => error_response(),
                };
                let _ = stream.write_all(response_body.as_bytes());
                let _ = stream.flush();
                return result;
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                oauth_log(format!("Loopback accept failed: {err}"));
                return Err(StorageError::from(err));
            }
        }
    }
}

fn read_oauth_callback(stream: &mut TcpStream) -> StorageResult<OAuthCallback> {
    let mut buffer = [0_u8; 4096];
    let read = stream.read(&mut buffer).map_err(StorageError::from)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let request_line = request.lines().next().unwrap_or_default();
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| StorageError::InvalidInput("invalid oauth callback request".to_string()))?;

    let query = path.split('?').nth(1).unwrap_or_default();
    let params = parse_query(query);

    if let Some(error) = params.get("error") {
        let description = params.get("error_description").map(String::as_str).unwrap_or("");
        oauth_log(format!(
            "Provider returned error in callback: {error} {description}"
        ));
        return Err(StorageError::InvalidInput(format!(
            "oauth provider returned error: {error}"
        )));
    }

    let code = params
        .get("code")
        .ok_or_else(|| StorageError::InvalidInput("oauth callback missing code".to_string()))?
        .to_string();
    let state = params
        .get("state")
        .ok_or_else(|| StorageError::InvalidInput("oauth callback missing state".to_string()))?
        .to_string();

    Ok(OAuthCallback { code, state })
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or_default();
            Some((
                urlencoding::decode(key).unwrap_or_default().into_owned(),
                urlencoding::decode(value).unwrap_or_default().into_owned(),
            ))
        })
        .collect()
}

fn success_response() -> String {
    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
     <html><body><h1>BuilderBoard</h1><p>Authentication complete. You can close this tab.</p></body></html>"
        .to_string()
}

fn error_response() -> String {
    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
     <html><body><h1>BuilderBoard</h1><p>Authentication failed. Return to the app and try again.</p></body></html>"
        .to_string()
}

pub fn generate_code_verifier() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_code_challenge(code_verifier: &str) -> StorageResult<String> {
    let digest = Sha256::digest(code_verifier.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(digest))
}

pub fn generate_state() -> String {
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn build_authorization_url(
    authorization_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &[String],
    state: &str,
    code_challenge: &str,
) -> StorageResult<String> {
    let scope = scopes.join(" ");
    let query = format!(
        "response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&scope),
        urlencoding::encode(state),
        urlencoding::encode(code_challenge),
    );
    Ok(format!("{authorization_url}?{query}"))
}

fn bind_loopback_listener() -> StorageResult<TcpListener> {
    for port in 49152..65535 {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if let Ok(listener) = TcpListener::bind(address) {
            return Ok(listener);
        }
    }

    Err(StorageError::InvalidInput(
        "failed to bind loopback callback port".to_string(),
    ))
}

pub fn resolve_google_credentials(_provider_id: &str) -> StorageResult<GoogleOAuthCredentials> {
    let client_id = std::env::var(GOOGLE_CLIENT_ID_ENV).map_err(|_| {
        StorageError::InvalidInput(format!(
            "missing Google OAuth client id; set {GOOGLE_CLIENT_ID_ENV}"
        ))
    })?;
    let client_secret = std::env::var(GOOGLE_CLIENT_SECRET_ENV).map_err(|_| {
        StorageError::InvalidInput(format!(
            "missing Google OAuth client secret; set {GOOGLE_CLIENT_SECRET_ENV} (required for Desktop App token exchange)"
        ))
    })?;

    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err(StorageError::InvalidInput(
            "Google OAuth client id and client secret must not be empty".to_string(),
        ));
    }

    Ok(GoogleOAuthCredentials {
        client_id,
        client_secret,
    })
}

fn map_oauth_error(provider_id: &str, error: StorageError) -> OAuthErrorEvent {
    let (error_code, message) = match &error {
        StorageError::InvalidInput(message) if message.contains("state mismatch") => {
            ("state_mismatch", "Authentication failed. Please try again.")
        }
        StorageError::InvalidInput(message) if message.contains("timed out") => {
            ("timeout", "Authentication timed out.")
        }
        StorageError::InvalidInput(message) if message.contains("cancelled") => {
            ("cancelled", "Authentication cancelled.")
        }
        StorageError::InvalidInput(message) if message.contains("token exchange") => {
            ("token_exchange_failed", "Could not connect account.")
        }
        _ => ("oauth_failed", "Could not connect account."),
    };

    OAuthErrorEvent {
        provider_id: provider_id.to_string(),
        error_code: error_code.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    use chrono::{Duration as ChronoDuration, Utc};
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::auth::commands::oauth_start_for_test;
    use crate::auth::credential_service::{CredentialService, OAuthCredential};
    use crate::storage::commands::account_disconnect_with_service;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::error::StorageResult;
    use crate::storage::repositories::accounts::AccountRepository;

    #[derive(Clone, Default)]
    struct MockBrowser {
        opened: Arc<Mutex<Vec<String>>>,
    }

    impl SystemBrowser for MockBrowser {
        fn open_url(&self, url: &str) -> StorageResult<()> {
            self.opened
                .lock()
                .map_err(|_| StorageError::InvalidInput("mock browser lock poisoned".to_string()))?
                .push(url.to_string());
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockHttpClient {
        token_response: TokenResponse,
        userinfo_response: UserInfoResponse,
        refresh_response: TokenResponse,
        last_exchange: Arc<Mutex<Option<(String, String, String)>>>,
        last_refresh: Arc<Mutex<Option<String>>>,
    }

    impl MockHttpClient {
        fn google_defaults() -> Self {
            Self {
                token_response: TokenResponse {
                    access_token: "access-token".to_string(),
                    refresh_token: Some("refresh-token".to_string()),
                    token_type: Some("Bearer".to_string()),
                    expires_in: Some(3600),
                    scope: Some("openid email".to_string()),
                },
                userinfo_response: UserInfoResponse {
                    sub: "google-subject".to_string(),
                    email: Some("user@example.com".to_string()),
                    name: Some("Google User".to_string()),
                },
                refresh_response: TokenResponse {
                    access_token: "refreshed-access".to_string(),
                    refresh_token: None,
                    token_type: Some("Bearer".to_string()),
                    expires_in: Some(3600),
                    scope: Some("openid email".to_string()),
                },
                last_exchange: Arc::new(Mutex::new(None)),
                last_refresh: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl OAuthHttpClient for MockHttpClient {
        fn exchange_code(
            &self,
            _token_url: &str,
            _client_id: &str,
            client_secret: &str,
            code: &str,
            code_verifier: &str,
            redirect_uri: &str,
        ) -> StorageResult<TokenResponse> {
            if client_secret.is_empty() {
                return Err(StorageError::InvalidInput(
                    "mock exchange requires client_secret".to_string(),
                ));
            }
            *self
                .last_exchange
                .lock()
                .map_err(|_| StorageError::InvalidInput("mock exchange lock poisoned".to_string()))? =
                Some((
                    code.to_string(),
                    code_verifier.to_string(),
                    redirect_uri.to_string(),
                ));
            Ok(self.token_response.clone())
        }

        fn refresh_token(
            &self,
            _token_url: &str,
            _client_id: &str,
            client_secret: &str,
            refresh_token: &str,
        ) -> StorageResult<TokenResponse> {
            if client_secret.is_empty() {
                return Err(StorageError::InvalidInput(
                    "mock refresh requires client_secret".to_string(),
                ));
            }
            *self
                .last_refresh
                .lock()
                .map_err(|_| StorageError::InvalidInput("mock refresh lock poisoned".to_string()))? =
                Some(refresh_token.to_string());
            Ok(self.refresh_response.clone())
        }

        fn fetch_userinfo(
            &self,
            _userinfo_url: &str,
            access_token: &str,
        ) -> StorageResult<UserInfoResponse> {
            if access_token != self.token_response.access_token {
                return Err(StorageError::InvalidInput("unexpected access token".to_string()));
            }
            Ok(self.userinfo_response.clone())
        }
    }

    fn setup_oauth_services(
        name: &str,
        http: MockHttpClient,
        browser: MockBrowser,
    ) -> StorageResult<(Arc<Database>, Arc<CredentialService>, OAuthService<MockHttpClient, MockBrowser>)> {
        let path = test_database_path(name)?;
        let _ = fs::remove_file(&path);
        let database = Arc::new(Database::initialize_at(path)?);
        let credentials = Arc::new(CredentialService::in_memory());
        let oauth = OAuthService::with_dependencies(
            http,
            browser,
            Box::new(|_| {
                Ok(GoogleOAuthCredentials {
                    client_id: "test-client-id".to_string(),
                    client_secret: "test-client-secret".to_string(),
                })
            }),
        );
        Ok((database, credentials, oauth))
    }

    fn query_param<'a>(url: &'a str, key: &str) -> &'a str {
        url.split('&')
            .find_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let found_key = parts.next()?;
                if found_key.ends_with(key) || found_key == key {
                    Some(parts.next().unwrap_or_default())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    fn send_callback(redirect_uri: &str, code: &str, state: &str) -> StorageResult<()> {
        let path = format!("/callback?code={code}&state={state}");
        let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        let mut stream = TcpStream::connect(redirect_uri.replace("http://", "").replace("/callback", ""))
            .or_else(|_| {
                let port = redirect_uri
                    .trim_start_matches("http://127.0.0.1:")
                    .split('/')
                    .next()
                    .unwrap_or("0");
                TcpStream::connect(format!("127.0.0.1:{port}"))
            })?;
        stream.write_all(request.as_bytes())?;
        Ok(())
    }

    #[test]
    fn pkce_challenge_uses_s256() -> StorageResult<()> {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier)?;
        let digest = Sha256::digest(verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(challenge, expected);
        assert!(verifier.len() >= 43);
        Ok(())
    }

    #[test]
    fn google_oauth_flow_completes_with_callback() -> StorageResult<()> {
        let http = MockHttpClient::google_defaults();
        let browser = MockBrowser::default();
        let browser_urls = Arc::clone(&browser.opened);
        let (database, credentials, oauth) =
            setup_oauth_services("oauth-flow-complete.db", http.clone(), browser)?;

        let (complete_tx, complete_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();

        let start = oauth_start_for_test(
            Arc::clone(&database),
            Arc::clone(&credentials),
            &oauth,
            "google",
            true,
            move |event| {
                let _ = complete_tx.send(event);
            },
            move |event| {
                let _ = error_tx.send(event);
            },
        )?;

        assert!(start.auth_url.contains("accounts.google.com"));
        assert!(start.auth_url.contains("code_challenge_method=S256"));
        assert_eq!(browser_urls.lock().unwrap().len(), 1);

        let decoded_url = start.auth_url.replace("%3A", ":").replace("%2F", "/");
        let redirect_uri = query_param(&decoded_url, "redirect_uri");
        let state = query_param(&start.auth_url, "state");
        let redirect_uri = urlencoding::decode(redirect_uri).unwrap_or_default().into_owned();
        let state = urlencoding::decode(state).unwrap_or_default().into_owned();

        thread::sleep(Duration::from_millis(100));
        send_callback(&redirect_uri, "auth-code", &state)?;

        let event = complete_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("oauth_complete should fire");
        assert_eq!(event.provider_id, "google");
        assert_eq!(event.label, "user@example.com");

        let exchange = http.last_exchange.lock().unwrap().clone().expect("token exchange");
        assert_eq!(exchange.0, "auth-code");
        assert!(!exchange.1.is_empty());

        let accounts = database.with_connection(|connection| AccountRepository::list_active(connection, Some("google")))?;
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].auth_type, "oauth");
        assert_eq!(accounts[0].external_email.as_deref(), Some("user@example.com"));
        assert!(credentials.credential_exists(
            &database.with_connection(|connection| AccountRepository::credential_ref(connection, &accounts[0].id))?,
        )?);

        assert!(error_rx.try_recv().is_err());
        Ok(())
    }

    #[test]
    fn oauth_rejects_state_mismatch() -> StorageResult<()> {
        let http = MockHttpClient::google_defaults();
        let browser = MockBrowser::default();
        let (database, credentials, oauth) =
            setup_oauth_services("oauth-state-mismatch.db", http, browser)?;

        let (error_tx, error_rx) = mpsc::channel();
        let start = oauth_start_for_test(
            Arc::clone(&database),
            Arc::clone(&credentials),
            &oauth,
            "google",
            false,
            |_| {},
            move |event| {
                let _ = error_tx.send(event);
            },
        )?;

        let decoded_url = start.auth_url.replace("%3A", ":").replace("%2F", "/");
        let redirect_uri = query_param(&decoded_url, "redirect_uri");
        let redirect_uri = urlencoding::decode(redirect_uri).unwrap_or_default().into_owned();
        thread::sleep(Duration::from_millis(100));
        send_callback(&redirect_uri, "auth-code", "wrong-state")?;

        let event = error_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("oauth_error should fire");
        assert_eq!(event.error_code, "state_mismatch");

        let accounts = database.with_connection(|connection| AccountRepository::list_active(connection, Some("google")))?;
        assert!(accounts.is_empty());
        Ok(())
    }

    #[test]
    fn oauth_refresh_updates_keychain_and_account() -> StorageResult<()> {
        let http = MockHttpClient::google_defaults();
        let browser = MockBrowser::default();
        let (database, credentials, oauth) =
            setup_oauth_services("oauth-refresh.db", http.clone(), browser)?;

        let credential_ref = CredentialService::generate_credential_ref();
        let expiring = OAuthCredential {
            access_token: "old-access".to_string(),
            refresh_token: "refresh-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: (Utc::now() + ChronoDuration::minutes(1)).to_rfc3339(),
        };
        credentials.store_oauth_credential(&credential_ref, "Google", "google", &expiring)?;

        let account_id = database.with_connection(|connection| {
            AccountRepository::create_oauth_account(
                connection,
                "google",
                "Google",
                &credential_ref,
                "subject",
                Some("user@example.com"),
                &expiring.expires_at,
                Some("openid email"),
                true,
            )
        })?
        .id;

        let refreshed = oauth.refresh_oauth_access_token(&database, &credentials, &account_id)?;
        assert_eq!(refreshed, "refreshed-access");
        assert_eq!(
            http.last_refresh.lock().unwrap().as_deref(),
            Some("refresh-token")
        );

        let stored = credentials.read_oauth_credential(&credential_ref)?;
        assert_eq!(stored.access_token, "refreshed-access");
        assert_eq!(stored.refresh_token, "refresh-token");

        let status = database.with_connection(|connection| AccountRepository::get_status(connection, &account_id))?;
        assert_eq!(status.status, "active");
        assert!(status.token_expires_at.is_some());
        Ok(())
    }

    #[test]
    fn oauth_disconnect_removes_keychain_entry() -> StorageResult<()> {
        let http = MockHttpClient::google_defaults();
        let browser = MockBrowser::default();
        let (database, credentials, _oauth) =
            setup_oauth_services("oauth-disconnect.db", http, browser)?;

        let credential_ref = CredentialService::generate_credential_ref();
        credentials.store_oauth_credential(
            &credential_ref,
            "Google",
            "google",
            &OAuthCredential {
                access_token: "access".to_string(),
                refresh_token: "refresh".to_string(),
                token_type: "Bearer".to_string(),
                expires_at: (Utc::now() + ChronoDuration::hours(1)).to_rfc3339(),
            },
        )?;

        let account = database.with_connection(|connection| {
            AccountRepository::create_oauth_account(
                connection,
                "google",
                "Google",
                &credential_ref,
                "subject",
                Some("user@example.com"),
                &(Utc::now() + ChronoDuration::hours(1)).to_rfc3339(),
                Some("openid"),
                true,
            )
        })?;

        let account_id = account.id;
        assert!(credentials.credential_exists(&credential_ref)?);
        account_disconnect_with_service(&database, &credentials, account_id.clone())?;
        assert!(!credentials.credential_exists(&credential_ref)?);

        let status = database.with_connection(|connection| AccountRepository::get_status(connection, &account_id))?;
        assert_eq!(status.status, "revoked");
        Ok(())
    }

    #[test]
    fn oauth_cancel_emits_cancelled_error() -> StorageResult<()> {
        let http = MockHttpClient::google_defaults();
        let browser = MockBrowser::default();
        let (database, credentials, oauth) =
            setup_oauth_services("oauth-cancel.db", http, browser)?;

        let (error_tx, error_rx) = mpsc::channel();
        let _start = oauth_start_for_test(
            Arc::clone(&database),
            Arc::clone(&credentials),
            &oauth,
            "google",
            false,
            |_| {},
            move |event| {
                let _ = error_tx.send(event);
            },
        )?;

        oauth.oauth_cancel("google")?;
        let event = error_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("oauth_error should fire on cancel");
        assert_eq!(event.error_code, "cancelled");
        Ok(())
    }
}