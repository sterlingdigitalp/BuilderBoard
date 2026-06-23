pub mod commands;
pub mod credential_service;
pub mod oauth_service;

pub use credential_service::{
    ApiKeyCredential, CredentialPayload, CredentialService, CredentialStore,
    KeychainCredentialStore, MemoryCredentialStore, OAuthCredential, KEYCHAIN_SERVICE,
};
pub use oauth_service::OAuthService;

pub trait AuthSessionStore {
    fn current_subject(&self) -> Option<&str>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialHandle {
    pub provider_id: String,
    pub account_id: String,
    pub auth_type: String,
    pub credential_ref: String,
    pub token_expires_at: Option<String>,
}

impl CredentialHandle {
    pub fn new(
        provider_id: impl Into<String>,
        account_id: impl Into<String>,
        auth_type: impl Into<String>,
        credential_ref: impl Into<String>,
        token_expires_at: Option<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            account_id: account_id.into(),
            auth_type: auth_type.into(),
            credential_ref: credential_ref.into(),
            token_expires_at,
        }
    }

    pub fn is_api_key(&self) -> bool {
        self.auth_type == "api_key"
    }

    pub fn is_oauth(&self) -> bool {
        self.auth_type == "oauth"
    }
}
