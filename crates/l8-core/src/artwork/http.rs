//! Minimal HTTP access for artwork and save-knowledge providers: one request at a time, polite pacing, bounded
//! downloads and an allow-list of hosts. Everything the providers need goes through [`HttpGet`] so
//! tests can substitute a fake.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use ureq::Agent;

use crate::error::CoreError;
use crate::redact::redact_secrets;

/// Largest image the app accepts, from a provider or from the user's disk.
pub const MAX_IMAGE_BYTES: u64 = 15 * 1024 * 1024;

const MIN_REQUEST_GAP: Duration = Duration::from_millis(1000);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_TEXT_BYTES: u64 = 4 * 1024 * 1024;
const USER_AGENT: &str = concat!("L8Play/", env!("CARGO_PKG_VERSION"), " (personal, non-commercial game library)");

/// Hosts the app downloads artwork from.
const ALLOWED_HOSTS: [&str; 5] = [
    "www.steamgriddb.com",
    "steamgriddb.com",
    "cdn2.steamgriddb.com",
    "cdn.steamgriddb.com",
    "cdn.cloudflare.steamstatic.com",
];

/// Failures a provider call can report; mapped to [`CoreError`] at the service boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpFailure {
    /// 401 / 403: the key is missing, wrong or revoked.
    Unauthorized,
    NotFound,
    /// 429: back off.
    RateLimited,
    Other(String),
}

impl From<HttpFailure> for CoreError {
    fn from(failure: HttpFailure) -> Self {
        match failure {
            HttpFailure::Unauthorized => CoreError::Auth("the artwork service refused the API key".into()),
            HttpFailure::NotFound => CoreError::NotFound("the artwork service has no such resource".into()),
            HttpFailure::RateLimited => CoreError::Busy("the artwork service is rate limiting requests".into()),
            HttpFailure::Other(message) => CoreError::External(message),
        }
    }
}

pub trait HttpGet {
    /// GET returning the response body as text. The bearer token, when given, is sent as an
    /// `Authorization` header and never logged.
    fn get_text(&self, url: &str, bearer: Option<&str>) -> Result<String, HttpFailure>;

    /// GET returning at most [`MAX_IMAGE_BYTES`] of body.
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpFailure>;

    /// Where a URL redirects to, without following it (`None` when it does not redirect).
    fn redirect_target(&self, _url: &str) -> Result<Option<String>, HttpFailure> {
        Ok(None)
    }

    /// GET returning text that only follows redirects to the same host (at most a few), so a page
    /// that moves inside one site works but can never send the request elsewhere.
    fn get_text_same_host(&self, url: &str) -> Result<String, HttpFailure> {
        self.get_text(url, None)
    }
}

/// Other read-only services the app queries (never for image downloads): save-location knowledge and
/// the public Steam Community profile and the Steam Web API (the owned-games list).
const KNOWLEDGE_HOSTS: [&str; 3] = ["www.pcgamingwiki.com", "steamcommunity.com", "api.steampowered.com"];

/// Redirects followed inside one host by [`HttpGet::get_text_same_host`].
const MAX_SAME_HOST_REDIRECTS: usize = 3;

/// A transport failure with its text scrubbed: a request URL may carry a key (`?key=…`), and the text of a
/// failure ends up in logs and UI messages, so no secret may survive in it.
fn transport_failure(what: &str, error: &dyn std::fmt::Display) -> HttpFailure {
    HttpFailure::Other(redact_secrets(&format!("{what}: {error}")))
}

fn host_of(url: &str) -> Option<&str> {
    url.strip_prefix("https://").map(|rest| rest.split(['/', '?', '#']).next().unwrap_or_default())
}

/// Artwork downloads: only the artwork providers' own hosts.
pub fn is_allowed_url(url: &str) -> bool {
    host_of(url).is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

/// Every request the transport may make: artwork hosts plus the knowledge hosts.
fn is_transport_allowed(url: &str) -> bool {
    host_of(url).is_some_and(|host| ALLOWED_HOSTS.contains(&host) || KNOWLEDGE_HOSTS.contains(&host))
}

/// A redirect destination, kept only when it stays on `host` over https (absolute or site-relative).
fn same_host_target(host: &str, location: &str) -> Option<String> {
    if location.starts_with('/') && !location.starts_with("//") {
        return Some(format!("https://{host}{location}"));
    }
    (host_of(location) == Some(host)).then(|| location.to_owned())
}

/// Percent-encodes a path segment or query value (RFC 3986 unreserved characters stay as they are).
pub fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Decodes `%XX` escapes (and `+` as a space); malformed escapes are kept as they are.
pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex =
                    std::str::from_utf8(&bytes[index + 1..index + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                if let Some(byte) = hex {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(b'%');
                    index += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `ureq` client with a 1 request/second pace shared by every call.
pub struct UreqHttp {
    agent: Agent,
    /// Same settings, but a redirect is reported instead of followed.
    no_redirect: Agent,
    last_request: Mutex<Option<Instant>>,
}

impl UreqHttp {
    pub fn new() -> Self {
        let agent: Agent =
            Agent::config_builder().timeout_global(Some(REQUEST_TIMEOUT)).user_agent(USER_AGENT).build().into();
        let no_redirect: Agent = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .build()
            .into();
        Self { agent, no_redirect, last_request: Mutex::new(None) }
    }

    fn pace(&self) {
        let Ok(mut last) = self.last_request.lock() else { return };
        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < MIN_REQUEST_GAP {
                std::thread::sleep(MIN_REQUEST_GAP - elapsed);
            }
        }
        *last = Some(Instant::now());
    }

    fn request(&self, url: &str, bearer: Option<&str>) -> Result<ureq::http::Response<ureq::Body>, HttpFailure> {
        if !is_transport_allowed(url) {
            return Err(HttpFailure::Other("the address is not an allowed host".into()));
        }
        self.pace();
        let mut request = self.agent.get(url);
        if let Some(token) = bearer {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        request.call().map_err(|error| match error {
            ureq::Error::StatusCode(401 | 403) => HttpFailure::Unauthorized,
            ureq::Error::StatusCode(404) => HttpFailure::NotFound,
            ureq::Error::StatusCode(429) => HttpFailure::RateLimited,
            ureq::Error::StatusCode(code) => HttpFailure::Other(format!("the server answered HTTP {code}")),
            other => transport_failure("network error", &other),
        })
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpGet for UreqHttp {
    fn redirect_target(&self, url: &str) -> Result<Option<String>, HttpFailure> {
        if !is_transport_allowed(url) {
            return Err(HttpFailure::Other("the address is not an allowed host".into()));
        }
        self.pace();
        let response = self.no_redirect.get(url).call().map_err(|error| transport_failure("network error", &error))?;
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(None);
        }
        Ok(response.headers().get("location").and_then(|value| value.to_str().ok()).map(str::to_owned))
    }

    fn get_text_same_host(&self, url: &str) -> Result<String, HttpFailure> {
        let host = host_of(url).ok_or_else(|| HttpFailure::Other("the address is not an https address".into()))?;
        let mut current = url.to_owned();
        for _ in 0..=MAX_SAME_HOST_REDIRECTS {
            if !is_transport_allowed(&current) {
                return Err(HttpFailure::Other("the address is not an allowed host".into()));
            }
            self.pace();
            let mut response =
                self.no_redirect.get(&current).call().map_err(|error| transport_failure("network error", &error))?;
            match response.status().as_u16() {
                200..=299 => {
                    return response
                        .body_mut()
                        .with_config()
                        .limit(MAX_TEXT_BYTES)
                        .read_to_string()
                        .map_err(|error| transport_failure("could not read the response", &error));
                }
                301 | 302 | 303 | 307 | 308 => {
                    let location = response
                        .headers()
                        .get("location")
                        .and_then(|value| value.to_str().ok())
                        .ok_or_else(|| HttpFailure::Other("a redirect without a destination".into()))?;
                    current = same_host_target(host, location)
                        .ok_or_else(|| HttpFailure::Other("the redirect leaves the site".into()))?;
                }
                401 | 403 => return Err(HttpFailure::Unauthorized),
                404 => return Err(HttpFailure::NotFound),
                429 => return Err(HttpFailure::RateLimited),
                code => return Err(HttpFailure::Other(format!("the server answered HTTP {code}"))),
            }
        }
        Err(HttpFailure::Other("too many redirects".into()))
    }

    fn get_text(&self, url: &str, bearer: Option<&str>) -> Result<String, HttpFailure> {
        let mut response = self.request(url, bearer)?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_TEXT_BYTES)
            .read_to_string()
            .map_err(|error| transport_failure("could not read the response", &error))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpFailure> {
        let mut response = self.request(url, None)?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_IMAGE_BYTES)
            .read_to_vec()
            .map_err(|error| transport_failure("could not download the image", &error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_artwork_hosts_are_allowed() {
        assert!(is_allowed_url("https://cdn2.steamgriddb.com/grid/abc.png"));
        assert!(is_allowed_url("https://www.steamgriddb.com/api/v2/games/steam/1"));
        assert!(!is_allowed_url("http://cdn2.steamgriddb.com/grid/abc.png"));
        assert!(!is_allowed_url("https://evil.example/cdn2.steamgriddb.com/a.png"));
        assert!(!is_allowed_url("https://cdn2.steamgriddb.com.evil.example/a.png"));
        assert!(!is_allowed_url("file:///C:/Windows/win.ini"));
    }

    #[test]
    fn failure_text_never_carries_a_key() {
        let key = "0123456789ABCDEF0123456789ABCDEF";
        let message = format!("error sending request for url (https://api.steampowered.com/x/?key={key}&steamid=1)");
        let HttpFailure::Other(text) = transport_failure("network error", &message) else {
            panic!("expected a transport failure");
        };
        assert!(!text.contains(key), "{text}");
        assert!(text.contains("[REDACTED]"));
        // A refused address is reported without echoing the URL at all.
        let http = UreqHttp::new();
        let url = format!("https://evil.example/path?key={key}");
        for failure in [http.get_text(&url, None), http.get_text_same_host(&url)] {
            assert!(!format!("{failure:?}").contains(key));
        }
    }

    #[test]
    fn redirects_stay_on_the_same_host() {
        let host = "steamcommunity.com";
        assert_eq!(
            same_host_target(host, "/id/someone/games?xml=1").as_deref(),
            Some("https://steamcommunity.com/id/someone/games?xml=1")
        );
        assert_eq!(
            same_host_target(host, "https://steamcommunity.com/x").as_deref(),
            Some("https://steamcommunity.com/x")
        );
        for bad in [
            "https://evil.example/x",
            "https://steamcommunity.com.evil.example/x",
            "http://steamcommunity.com/x",
            "//evil.example/x",
            "steamcommunity.com/x",
        ] {
            assert!(same_host_target(host, bad).is_none(), "{bad}");
        }
        assert!(is_transport_allowed("https://steamcommunity.com/profiles/1/games"));
        assert!(is_transport_allowed("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/"));
        assert!(!is_allowed_url("https://api.steampowered.com/x.png"), "never an image host");
        assert!(!is_allowed_url("https://steamcommunity.com/a.png"), "never an image host");
    }

    #[test]
    fn percent_encoding_keeps_unreserved_characters() {
        assert_eq!(percent_encode("The Last of Us: Part I"), "The%20Last%20of%20Us%3A%20Part%20I");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(percent_encode("Pokémon"), "Pok%C3%A9mon");
    }
}
