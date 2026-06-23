use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};

pub const KEYCHAIN_SERVICE: &str = "com.builderboard.app";
const REFRESH_BUFFER: Duration = Duration::minutes(5);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKeyCredential {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CredentialPayload {
    ApiKey(ApiKeyCredential),
    OAuth(OAuthCredential),
}

impl CredentialPayload {
    pub fn from_json(raw: &str) -> StorageResult<Self> {
        serde_json::from_str(raw).map_err(StorageError::from)
    }
}

pub trait CredentialStore: Send + Sync {
    fn store_payload(
        &self,
        credential_ref: &str,
        label: &str,
        provider_id: &str,
        payload: &str,
    ) -> StorageResult<()>;
    fn read_payload(&self, credential_ref: &str) -> StorageResult<String>;
    fn delete_credential(&self, credential_ref: &str) -> StorageResult<()>;
    fn contains_credential(&self, credential_ref: &str) -> StorageResult<bool>;
}

pub struct CredentialService {
    store: Box<dyn CredentialStore>,
}

impl CredentialService {
    pub fn with_store(store: Box<dyn CredentialStore>) -> Self {
        Self { store }
    }

    pub fn keychain() -> Self {
        Self::with_store(Box::new(KeychainCredentialStore))
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::with_store(Box::new(MemoryCredentialStore::default()))
    }

    pub fn generate_credential_ref() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn store_api_key(
        &self,
        credential_ref: &str,
        label: &str,
        provider_id: &str,
        api_key: &str,
    ) -> StorageResult<()> {
        if api_key.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "api key must not be empty".to_string(),
            ));
        }

        let payload = CredentialPayload::ApiKey(ApiKeyCredential {
            api_key: api_key.to_string(),
        });
        let serialized = serde_json::to_string(&payload)?;
        self.store
            .store_payload(credential_ref, label, provider_id, &serialized)
    }

    pub fn store_oauth_credential(
        &self,
        credential_ref: &str,
        label: &str,
        provider_id: &str,
        credential: &OAuthCredential,
    ) -> StorageResult<()> {
        if credential.access_token.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "oauth access token must not be empty".to_string(),
            ));
        }

        let payload = CredentialPayload::OAuth(credential.clone());
        let serialized = serde_json::to_string(&payload)?;
        self.store
            .store_payload(credential_ref, label, provider_id, &serialized)
    }

    pub fn read_oauth_credential(&self, credential_ref: &str) -> StorageResult<OAuthCredential> {
        let raw = self.store.read_payload(credential_ref)?;
        match CredentialPayload::from_json(&raw)? {
            CredentialPayload::OAuth(credential) => Ok(credential),
            CredentialPayload::ApiKey(_) => Err(StorageError::InvalidInput(
                "credential is not an oauth token".to_string(),
            )),
        }
    }

    pub fn delete_credential(&self, credential_ref: &str) -> StorageResult<()> {
        self.store.delete_credential(credential_ref)
    }

    pub fn credential_exists(&self, credential_ref: &str) -> StorageResult<bool> {
        self.store.contains_credential(credential_ref)
    }

    pub fn oauth_access_token_needs_refresh(credential: &OAuthCredential) -> StorageResult<bool> {
        let expires_at = DateTime::parse_from_rfc3339(&credential.expires_at)
            .map_err(|err| StorageError::InvalidInput(format!("invalid expires_at: {err}")))?
            .with_timezone(&Utc);
        Ok(expires_at <= Utc::now() + REFRESH_BUFFER)
    }

    pub fn oauth_credential_from_token_response(
        access_token: String,
        refresh_token: Option<String>,
        token_type: Option<String>,
        expires_in: Option<i64>,
        existing_refresh_token: Option<&str>,
    ) -> StorageResult<OAuthCredential> {
        let refresh_token = refresh_token
            .or_else(|| existing_refresh_token.map(str::to_string))
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| {
                StorageError::InvalidInput("oauth refresh token is required".to_string())
            })?;

        let expires_at = Utc::now()
            + Duration::seconds(expires_in.unwrap_or(3600).max(1));
        Ok(OAuthCredential {
            access_token,
            refresh_token,
            token_type: token_type.unwrap_or_else(|| "Bearer".to_string()),
            expires_at: expires_at.to_rfc3339(),
        })
    }
}

pub struct KeychainCredentialStore;

impl CredentialStore for KeychainCredentialStore {
    fn store_payload(
        &self,
        credential_ref: &str,
        label: &str,
        provider_id: &str,
        payload: &str,
    ) -> StorageResult<()> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, credential_ref).map_err(|err| {
            StorageError::Keychain(format!("failed to open keychain entry: {err}"))
        })?;

        entry.set_password(payload).map_err(|err| {
            StorageError::Keychain(format!(
                "failed to store credential for BuilderBoard:{provider_id}:{label}: {err}"
            ))
        })?;

        Ok(())
    }

    fn read_payload(&self, credential_ref: &str) -> StorageResult<String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, credential_ref).map_err(|err| {
            StorageError::Keychain(format!("failed to open keychain entry: {err}"))
        })?;

        entry.get_password().map_err(|err| match err {
            keyring::Error::NoEntry => {
                StorageError::NotFound(format!("credential {credential_ref} not found"))
            }
            other => StorageError::Keychain(format!("failed to read keychain credential: {other}")),
        })
    }

    fn delete_credential(&self, credential_ref: &str) -> StorageResult<()> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, credential_ref).map_err(|err| {
            StorageError::Keychain(format!("failed to open keychain entry: {err}"))
        })?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(StorageError::Keychain(format!(
                "failed to delete keychain credential: {err}"
            ))),
        }
    }

    fn contains_credential(&self, credential_ref: &str) -> StorageResult<bool> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, credential_ref).map_err(|err| {
            StorageError::Keychain(format!("failed to open keychain entry: {err}"))
        })?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(StorageError::Keychain(format!(
                "failed to read keychain credential: {err}"
            ))),
        }
    }
}

#[derive(Default)]
pub struct MemoryCredentialStore {
    entries: Mutex<HashMap<String, String>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn store_payload(
        &self,
        credential_ref: &str,
        _label: &str,
        _provider_id: &str,
        payload: &str,
    ) -> StorageResult<()> {
        self.entries
            .lock()
            .map_err(|_| StorageError::Keychain("memory credential store lock poisoned".to_string()))?
            .insert(credential_ref.to_string(), payload.to_string());
        Ok(())
    }

    fn read_payload(&self, credential_ref: &str) -> StorageResult<String> {
        self.entries
            .lock()
            .map_err(|_| StorageError::Keychain("memory credential store lock poisoned".to_string()))?
            .get(credential_ref)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(format!("credential {credential_ref} not found")))
    }

    fn delete_credential(&self, credential_ref: &str) -> StorageResult<()> {
        self.entries
            .lock()
            .map_err(|_| StorageError::Keychain("memory credential store lock poisoned".to_string()))?
            .remove(credential_ref);
        Ok(())
    }

    fn contains_credential(&self, credential_ref: &str) -> StorageResult<bool> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| StorageError::Keychain("memory credential store lock poisoned".to_string()))?
            .contains_key(credential_ref))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_credential_round_trip() -> StorageResult<()> {
        let service = CredentialService::in_memory();
        let credential_ref = CredentialService::generate_credential_ref();
        let credential = OAuthCredential {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: "2026-06-23T16:00:00Z".to_string(),
        };

        service.store_oauth_credential(&credential_ref, "Google", "google", &credential)?;
        let loaded = service.read_oauth_credential(&credential_ref)?;
        assert_eq!(loaded, credential);
        Ok(())
    }

    #[test]
    fn refresh_needed_within_buffer() -> StorageResult<()> {
        let credential = OAuthCredential {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: (Utc::now() + Duration::minutes(2)).to_rfc3339(),
        };
        assert!(CredentialService::oauth_access_token_needs_refresh(&credential)?);
        Ok(())
    }
}