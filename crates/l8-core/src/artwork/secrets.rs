//! Secret storage. API keys live only in the Windows Credential Manager (entry
//! `L8Play` / `<name>`); they never enter the database, the logs or the WebView after being saved.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{CoreError, CoreResult};

pub const STEAMGRIDDB_KEY: &str = "steamgriddb";
/// The user's own Steam Web API key (lists the games their account owns).
pub const STEAM_WEB_API_KEY: &str = "steam-web-api";

const CREDENTIAL_SERVICE: &str = "L8Play";

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> CoreResult<Option<String>>;
    fn set(&self, name: &str, value: &str) -> CoreResult<()>;
    /// Deleting a secret that does not exist is not an error.
    fn delete(&self, name: &str) -> CoreResult<()>;
}

/// Windows Credential Manager through the `keyring` crate.
#[derive(Debug, Default)]
pub struct CredentialManagerStore;

fn entry(name: &str) -> CoreResult<keyring::Entry> {
    keyring::Entry::new(CREDENTIAL_SERVICE, name).map_err(store_failure)
}

/// Error text never includes the secret: `keyring` errors describe the store, not the value.
fn store_failure(error: keyring::Error) -> CoreError {
    CoreError::External(format!("the Windows credential store failed: {error}"))
}

impl SecretStore for CredentialManagerStore {
    fn get(&self, name: &str) -> CoreResult<Option<String>> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(store_failure(error)),
        }
    }

    fn set(&self, name: &str, value: &str) -> CoreResult<()> {
        entry(name)?.set_password(value).map_err(store_failure)
    }

    fn delete(&self, name: &str) -> CoreResult<()> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(store_failure(error)),
        }
    }
}

/// In-memory store for tests.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    values: Mutex<HashMap<String, String>>,
}

impl MemorySecretStore {
    fn values(&self) -> CoreResult<std::sync::MutexGuard<'_, HashMap<String, String>>> {
        self.values.lock().map_err(|_| CoreError::LockPoisoned("secret store"))
    }
}

impl SecretStore for MemorySecretStore {
    fn get(&self, name: &str) -> CoreResult<Option<String>> {
        Ok(self.values()?.get(name).cloned())
    }

    fn set(&self, name: &str, value: &str) -> CoreResult<()> {
        self.values()?.insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> CoreResult<()> {
        self.values()?.remove(name);
        Ok(())
    }
}
