use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};

pub const KEYCHAIN_SERVICE: &str = "com.builderboard.app";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCredential {
    pub api_key: String,
}

pub trait CredentialStore: Send + Sync {
    fn store_api_key(&self, credential_ref: &str, label: &str, provider_id: &str, api_key: &str)
        -> StorageResult<()>;
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

        self.store
            .store_api_key(credential_ref, label, provider_id, api_key)
    }

    pub fn delete_credential(&self, credential_ref: &str) -> StorageResult<()> {
        self.store.delete_credential(credential_ref)
    }

    pub fn credential_exists(&self, credential_ref: &str) -> StorageResult<bool> {
        self.store.contains_credential(credential_ref)
    }
}

pub struct KeychainCredentialStore;

impl CredentialStore for KeychainCredentialStore {
    fn store_api_key(
        &self,
        credential_ref: &str,
        label: &str,
        provider_id: &str,
        api_key: &str,
    ) -> StorageResult<()> {
        let payload = ApiKeyCredential {
            api_key: api_key.to_string(),
        };
        let serialized = serde_json::to_string(&payload).map_err(|err| {
            StorageError::Keychain(format!("failed to encode credential payload: {err}"))
        })?;

        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, credential_ref).map_err(|err| {
            StorageError::Keychain(format!("failed to open keychain entry: {err}"))
        })?;

        entry.set_password(&serialized).map_err(|err| {
            StorageError::Keychain(format!(
                "failed to store credential for BuilderBoard:{provider_id}:{label}: {err}"
            ))
        })?;

        Ok(())
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
    fn store_api_key(
        &self,
        credential_ref: &str,
        _label: &str,
        _provider_id: &str,
        api_key: &str,
    ) -> StorageResult<()> {
        let payload = ApiKeyCredential {
            api_key: api_key.to_string(),
        };
        let serialized = serde_json::to_string(&payload)?;
        self.entries
            .lock()
            .map_err(|_| StorageError::Keychain("memory credential store lock poisoned".to_string()))?
            .insert(credential_ref.to_string(), serialized);
        Ok(())
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