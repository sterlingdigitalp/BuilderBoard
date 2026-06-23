pub mod credential_service;

pub use credential_service::{
    CredentialService, CredentialStore, KeychainCredentialStore, MemoryCredentialStore,
    KEYCHAIN_SERVICE,
};

pub trait AuthSessionStore {
    fn current_subject(&self) -> Option<&str>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialHandle {
    pub provider_id: String,
    pub account_id: String,
    pub auth_type: String,
    pub credential_ref: String,
}

impl CredentialHandle {
    pub fn new(
        provider_id: impl Into<String>,
        account_id: impl Into<String>,
        auth_type: impl Into<String>,
        credential_ref: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            account_id: account_id.into(),
            auth_type: auth_type.into(),
            credential_ref: credential_ref.into(),
        }
    }
}
