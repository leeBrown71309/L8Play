//! Which Google OAuth client the app uses.
//!
//! Resolution order: the client the user saved (Advanced option), then the project's embedded
//! client compiled in from `L8_GOOGLE_CLIENT_ID` / `L8_GOOGLE_CLIENT_SECRET`, then none (the user
//! must provide one). The secret of a desktop client is not confidential by Google's own design,
//! but it is still never logged, never put in a browser URL and never sent to the WebView.

use serde::Serialize;
use ts_rs::TS;

use crate::artwork::secrets::SecretStore;
use crate::cloud::oauth::{SECRET_CLIENT_ID, SECRET_CLIENT_SECRET};
use crate::error::CoreResult;

const EMBEDDED_ID: Option<&str> = option_env!("L8_GOOGLE_CLIENT_ID");
const EMBEDDED_SECRET: Option<&str> = option_env!("L8_GOOGLE_CLIENT_SECRET");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ClientSource {
    /// The user's own Google Cloud project, saved in the Credential Manager.
    Custom,
    /// The project's client, compiled into this build.
    Embedded,
    /// Nothing available: the user must provide a client.
    None,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GoogleClient {
    pub id: String,
    pub secret: String,
    pub source: ClientSource,
}

// The secret must never reach a log through `{:?}`.
impl std::fmt::Debug for GoogleClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("GoogleClient").field("source", &self.source).finish_non_exhaustive()
    }
}

/// The client compiled into this build, if both values were provided and are not blank.
pub fn embedded_client() -> Option<(&'static str, &'static str)> {
    match (EMBEDDED_ID.map(str::trim), EMBEDDED_SECRET.map(str::trim)) {
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => Some((id, secret)),
        _ => None,
    }
}

/// Picks the client to use: custom, then `embedded`, then none.
pub fn resolve_client(secrets: &dyn SecretStore, embedded: Option<(&str, &str)>) -> CoreResult<Option<GoogleClient>> {
    let id = secrets.get(SECRET_CLIENT_ID)?.filter(|value| !value.trim().is_empty());
    let secret = secrets.get(SECRET_CLIENT_SECRET)?.filter(|value| !value.trim().is_empty());
    if let (Some(id), Some(secret)) = (id, secret) {
        return Ok(Some(GoogleClient { id, secret, source: ClientSource::Custom }));
    }
    Ok(embedded.map(|(id, secret)| GoogleClient {
        id: id.to_owned(),
        secret: secret.to_owned(),
        source: ClientSource::Embedded,
    }))
}

/// Resolution against this build's embedded client; also registers the secret with the log
/// redaction so it is masked even if it ever ends up in a message.
pub fn current_client(secrets: &dyn SecretStore) -> CoreResult<Option<GoogleClient>> {
    let client = resolve_client(secrets, embedded_client())?;
    if let Some(client) = &client {
        crate::redact::register_secret(&client.secret);
    }
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artwork::secrets::MemorySecretStore;

    const EMBEDDED: (&str, &str) = ("embedded.apps.googleusercontent.com", "embedded-secret");

    fn with_custom(secrets: &MemorySecretStore) -> CoreResult<()> {
        secrets.set(SECRET_CLIENT_ID, "mine.apps.googleusercontent.com")?;
        secrets.set(SECRET_CLIENT_SECRET, "my-secret")
    }

    #[test]
    fn a_custom_client_wins_over_the_embedded_one() -> CoreResult<()> {
        let secrets = MemorySecretStore::default();
        with_custom(&secrets)?;
        let client = resolve_client(&secrets, Some(EMBEDDED))?.ok_or(crate::CoreError::NotFound("client".into()))?;
        assert_eq!(client.source, ClientSource::Custom);
        assert_eq!(client.id, "mine.apps.googleusercontent.com");
        Ok(())
    }

    #[test]
    fn the_embedded_client_is_used_when_nothing_custom_is_saved() -> CoreResult<()> {
        let secrets = MemorySecretStore::default();
        let client = resolve_client(&secrets, Some(EMBEDDED))?.ok_or(crate::CoreError::NotFound("client".into()))?;
        assert_eq!(client.source, ClientSource::Embedded);
        assert_eq!(client.id, EMBEDDED.0);
        Ok(())
    }

    #[test]
    fn without_either_there_is_no_client() -> CoreResult<()> {
        assert_eq!(resolve_client(&MemorySecretStore::default(), None)?, None);
        Ok(())
    }

    #[test]
    fn a_half_saved_or_blank_custom_client_is_ignored() -> CoreResult<()> {
        let secrets = MemorySecretStore::default();
        secrets.set(SECRET_CLIENT_ID, "mine.apps.googleusercontent.com")?;
        let only_id = resolve_client(&secrets, Some(EMBEDDED))?.map(|client| client.source);
        assert_eq!(only_id, Some(ClientSource::Embedded));
        secrets.set(SECRET_CLIENT_SECRET, "   ")?;
        assert_eq!(resolve_client(&secrets, None)?, None);
        Ok(())
    }

    #[test]
    fn removing_the_custom_client_goes_back_to_the_embedded_one() -> CoreResult<()> {
        let secrets = MemorySecretStore::default();
        with_custom(&secrets)?;
        secrets.delete(SECRET_CLIENT_ID)?;
        secrets.delete(SECRET_CLIENT_SECRET)?;
        let source = resolve_client(&secrets, Some(EMBEDDED))?.map(|client| client.source);
        assert_eq!(source, Some(ClientSource::Embedded));
        Ok(())
    }

    #[test]
    fn the_secret_is_never_part_of_the_debug_output() {
        let client =
            GoogleClient { id: "id".into(), secret: "top-secret-value".into(), source: ClientSource::Embedded };
        assert!(!format!("{client:?}").contains("top-secret-value"));
    }

    #[test]
    fn the_authorization_url_never_contains_the_embedded_or_custom_secret() -> CoreResult<()> {
        use crate::cloud::oauth::AuthorizationFlow;
        for (id, secret) in [EMBEDDED, ("mine.apps.googleusercontent.com", "my-secret")] {
            let url = AuthorizationFlow::start(id, secret)?.authorization_url()?;
            assert!(url.contains(id), "the public client id is part of the URL");
            assert!(!url.contains(secret), "the client secret is not");
        }
        Ok(())
    }
}
