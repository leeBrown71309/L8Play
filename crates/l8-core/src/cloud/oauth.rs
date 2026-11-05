//! Google OAuth 2.0 for installed apps: authorization code + PKCE (S256) with a loopback redirect
//! on `127.0.0.1`. Scope is `drive.file` only (the app sees just the files it created). The user
//! supplies their own OAuth client (Cloud project without billing); client id/secret and the refresh
//! token live in the Windows Credential Manager, the access token only in memory.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::artwork::http::{percent_decode, percent_encode};
use crate::artwork::secrets::SecretStore;
use crate::cloud::transport::{Method, Request, Transport};
use crate::error::{CoreError, CoreResult};

pub const SECRET_CLIENT_ID: &str = "google_client_id";
pub const SECRET_CLIENT_SECRET: &str = "google_client_secret";
pub const SECRET_REFRESH_TOKEN: &str = "google_refresh_token";

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const CALLBACK_WAIT: Duration = Duration::from_secs(180);
/// Refresh a minute early so a token never expires mid-request.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);

/// RFC 4648 base64url without padding (PKCE challenge, random strings).
pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, byte)| acc | (u32::from(*byte) << (16 - 8 * i)));
        let symbols = chunk.len() + 1;
        for i in 0..symbols {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 0x3F) as usize]));
        }
    }
    out
}

/// `code_challenge` = BASE64URL(SHA256(verifier)).
pub fn code_challenge(verifier: &str) -> String {
    base64url(&Sha256::digest(verifier.as_bytes()))
}

fn random_token(byte_count: usize) -> CoreResult<String> {
    let mut bytes = vec![0u8; byte_count];
    getrandom::fill(&mut bytes).map_err(|error| CoreError::External(format!("no secure random source: {error}")))?;
    Ok(base64url(&bytes))
}

/// One authorization attempt: listening socket, PKCE verifier and anti-forgery state.
pub struct AuthorizationFlow {
    listener: TcpListener,
    verifier: String,
    state: String,
    client_id: String,
    client_secret: String,
}

impl AuthorizationFlow {
    pub fn start(client_id: &str, client_secret: &str) -> CoreResult<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        Ok(Self {
            listener,
            // 64 random bytes → 86 URL-safe characters (RFC 7636 allows 43–128).
            verifier: random_token(64)?,
            state: random_token(24)?,
            client_id: client_id.to_owned(),
            client_secret: client_secret.to_owned(),
        })
    }

    fn redirect_uri(&self) -> CoreResult<String> {
        Ok(format!("http://127.0.0.1:{}", self.listener.local_addr()?.port()))
    }

    /// The consent-page address to open in the user's browser.
    pub fn authorization_url(&self) -> CoreResult<String> {
        Ok(format!(
            "{AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}\
             &code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
            percent_encode(&self.client_id),
            percent_encode(&self.redirect_uri()?),
            percent_encode(SCOPE),
            code_challenge(&self.verifier),
            self.state
        ))
    }

    /// Waits for Google's redirect, checks `state`, and trades the code for tokens.
    pub fn finish(self, transport: &dyn Transport) -> CoreResult<TokenGrant> {
        let code = wait_for_code(&self.listener, &self.state)?;
        let form = format!(
            "client_id={}&client_secret={}&code={}&code_verifier={}&grant_type=authorization_code&redirect_uri={}",
            percent_encode(&self.client_id),
            percent_encode(&self.client_secret),
            percent_encode(&code),
            percent_encode(&self.verifier),
            percent_encode(&self.redirect_uri()?)
        );
        let response = post_form(transport, TOKEN_URL, &form)?;
        parse_grant(&response.text(), response.status)
    }
}

const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>L8Play</title>\
<body style='font-family:sans-serif;text-align:center;margin-top:20vh'>\
<h2>L8Play is connected to Google Drive.</h2><p>You can close this tab and go back to the app.</p></body>";
const PAGE_ERROR: &str = "<!doctype html><meta charset=utf-8><title>L8Play</title>\
<body style='font-family:sans-serif;text-align:center;margin-top:20vh'>\
<h2>The connection was not completed.</h2><p>Go back to L8Play and try again.</p></body>";

fn respond(mut stream: TcpStream, status: &str, body: &str) {
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The browser tab is a courtesy; a failed write changes nothing for the flow.
    let _ = stream.write_all(reply.as_bytes());
}

fn wait_for_code(listener: &TcpListener, expected_state: &str) -> CoreResult<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + CALLBACK_WAIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut request_line = String::new();
                BufReader::new(&stream).read_line(&mut request_line)?;
                match parse_callback(&request_line, expected_state) {
                    CallbackOutcome::Code(code) => {
                        respond(stream, "200 OK", PAGE_OK);
                        return Ok(code);
                    }
                    CallbackOutcome::Denied => {
                        respond(stream, "200 OK", PAGE_ERROR);
                        return Err(CoreError::Auth("the Google consent was cancelled".into()));
                    }
                    // Favicon requests and other noise: keep waiting for the real redirect.
                    CallbackOutcome::Ignore => respond(stream, "404 Not Found", PAGE_ERROR),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(CoreError::Busy("the Google sign-in timed out".into()));
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CallbackOutcome {
    Code(String),
    Denied,
    Ignore,
}

/// Reads `GET /?code=…&state=…` from the first request line.
fn parse_callback(request_line: &str, expected_state: &str) -> CallbackOutcome {
    let Some(target) = request_line.split_whitespace().nth(1) else { return CallbackOutcome::Ignore };
    let Some(query) = target.split_once('?').map(|(_, query)| query) else { return CallbackOutcome::Ignore };
    let mut code = None;
    let mut state = None;
    let mut error = false;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("code", value)) => code = Some(percent_decode(value)),
            Some(("state", value)) => state = Some(percent_decode(value)),
            Some(("error", _)) => error = true,
            _ => {}
        }
    }
    if state.as_deref() != Some(expected_state) {
        // A request without our anti-forgery state is never accepted.
        return CallbackOutcome::Ignore;
    }
    match (code, error) {
        (Some(code), false) if !code.is_empty() => CallbackOutcome::Code(code),
        _ => CallbackOutcome::Denied,
    }
}

fn post_form(transport: &dyn Transport, url: &str, form: &str) -> CoreResult<crate::cloud::transport::Response> {
    transport.send(
        &Request::new(Method::Post, url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form.as_bytes().to_vec()),
    )
}

/// Tokens returned by Google. The refresh token is only present on the first (consenting) grant.
#[derive(Debug, Clone)]
pub struct TokenGrant {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Duration,
}

fn parse_grant(body: &str, status: u16) -> CoreResult<TokenGrant> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| CoreError::External(format!("unexpected answer from Google: {error}")))?;
    if let Some(error) = json.get("error").and_then(|value| value.as_str()) {
        // `invalid_grant` = revoked or expired refresh token; `invalid_client` = wrong client id/secret.
        return match error {
            "invalid_grant" | "invalid_client" | "unauthorized_client" | "access_denied" => {
                Err(CoreError::Auth(format!("Google refused the credentials ({error})")))
            }
            other => Err(CoreError::External(format!("Google answered {other} (HTTP {status})"))),
        };
    }
    let access_token = json
        .get("access_token")
        .and_then(|value| value.as_str())
        .ok_or_else(|| CoreError::External("Google sent no access token".into()))?
        .to_owned();
    let expires = json.get("expires_in").and_then(serde_json::Value::as_u64).unwrap_or(3600);
    Ok(TokenGrant {
        access_token,
        refresh_token: json.get("refresh_token").and_then(|value| value.as_str()).map(str::to_owned),
        expires_in: Duration::from_secs(expires),
    })
}

/// Source of valid access tokens for the Drive client.
pub trait TokenSource: Send + Sync {
    fn access_token(&self) -> CoreResult<String>;
}

/// Refreshes access tokens from the stored refresh token and caches them until shortly before expiry.
pub struct GoogleTokens<'a> {
    secrets: &'a dyn SecretStore,
    transport: &'a dyn Transport,
    cache: &'a Mutex<Option<(String, Instant)>>,
}

impl<'a> GoogleTokens<'a> {
    pub fn new(
        secrets: &'a dyn SecretStore,
        transport: &'a dyn Transport,
        cache: &'a Mutex<Option<(String, Instant)>>,
    ) -> Self {
        Self { secrets, transport, cache }
    }
}

impl TokenSource for GoogleTokens<'_> {
    fn access_token(&self) -> CoreResult<String> {
        let mut cache = self.cache.lock().map_err(|_| CoreError::LockPoisoned("token cache"))?;
        if let Some((token, expires_at)) = cache.as_ref()
            && Instant::now() + EXPIRY_MARGIN < *expires_at
        {
            return Ok(token.clone());
        }
        let missing = |what: &str| CoreError::Auth(format!("no {what} is available; connect Google Drive again"));
        let refresh = self.secrets.get(SECRET_REFRESH_TOKEN)?.ok_or_else(|| missing("refresh token"))?;
        let client = crate::cloud::client::current_client(self.secrets)?.ok_or_else(|| missing("Google client"))?;
        let (client_id, client_secret) = (client.id, client.secret);
        let form = format!(
            "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
            percent_encode(&client_id),
            percent_encode(&client_secret),
            percent_encode(&refresh)
        );
        let response = post_form(self.transport, TOKEN_URL, &form)?;
        let grant = parse_grant(&response.text(), response.status)?;
        *cache = Some((grant.access_token.clone(), Instant::now() + grant.expires_in));
        Ok(grant.access_token)
    }
}

/// Revokes the refresh token at Google (best effort when disconnecting).
pub fn revoke(transport: &dyn Transport, token: &str) -> CoreResult<()> {
    let response = post_form(transport, REVOKE_URL, &format!("token={}", percent_encode(token)))?;
    if response.is_success() || response.status == 400 {
        // 400 = already revoked or expired: the goal (no valid token) is met either way.
        Ok(())
    } else {
        Err(CoreError::External(format!("Google could not revoke the token (HTTP {})", response.status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_the_rfc_7636_example() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn base64url_handles_every_remainder_length() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(&[0xFB, 0xFF]), "-_8");
    }

    #[test]
    fn the_authorization_url_carries_pkce_and_the_drive_file_scope_only() -> CoreResult<()> {
        let flow = AuthorizationFlow::start("my id.apps.googleusercontent.com", "secret")?;
        let url = flow.authorization_url()?;
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=my%20id.apps"));
        assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A"));
        assert!(!url.contains("secret"), "the client secret never appears in the browser URL");
        assert!(flow.verifier.len() >= 43 && flow.verifier.len() <= 128);
        Ok(())
    }

    #[test]
    fn the_callback_needs_the_matching_state() {
        let line = "GET /?code=4%2F0AbC&state=xyz&scope=drive.file HTTP/1.1";
        assert_eq!(parse_callback(line, "xyz"), CallbackOutcome::Code("4/0AbC".into()));
        assert_eq!(parse_callback(line, "other"), CallbackOutcome::Ignore);
        assert_eq!(parse_callback("GET /?error=access_denied&state=xyz HTTP/1.1", "xyz"), CallbackOutcome::Denied);
        assert_eq!(parse_callback("GET /favicon.ico HTTP/1.1", "xyz"), CallbackOutcome::Ignore);
        assert_eq!(parse_callback("", "xyz"), CallbackOutcome::Ignore);
    }

    #[test]
    fn token_answers_are_parsed_and_errors_classified() -> CoreResult<()> {
        let grant = parse_grant(r#"{"access_token":"a","expires_in":3599,"refresh_token":"r"}"#, 200)?;
        assert_eq!(grant.access_token, "a");
        assert_eq!(grant.refresh_token.as_deref(), Some("r"));
        assert_eq!(grant.expires_in, Duration::from_secs(3599));
        assert!(matches!(parse_grant(r#"{"error":"invalid_grant"}"#, 400), Err(CoreError::Auth(_))));
        assert!(matches!(parse_grant(r#"{"error":"invalid_client"}"#, 401), Err(CoreError::Auth(_))));
        assert!(matches!(parse_grant(r#"{"error":"server_error"}"#, 500), Err(CoreError::External(_))));
        assert!(matches!(parse_grant("<html>", 502), Err(CoreError::External(_))));
        Ok(())
    }

    #[test]
    fn percent_decoding_is_forgiving() {
        assert_eq!(percent_decode("4%2F0A+b"), "4/0A b");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
