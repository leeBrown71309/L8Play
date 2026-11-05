//! HTTP transport for the Google APIs: any method, bodies, response headers, streamed downloads.
//! Only Google's OAuth and Drive hosts are reachable, over HTTPS; status codes are returned to the
//! caller (not turned into errors) because the resumable-upload protocol uses 308 as a normal answer.

use std::io::Write;
use std::time::Duration;

use ureq::Agent;

use crate::error::{CoreError, CoreResult};

const ALLOWED_HOSTS: [&str; 4] =
    ["accounts.google.com", "oauth2.googleapis.com", "www.googleapis.com", "upload.googleapis.com"];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;
const USER_AGENT: &str = concat!("L8Play/", env!("CARGO_PKG_VERSION"), " (personal, non-commercial game library)");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self { method, url: url.into(), headers: Vec::new(), body: Vec::new() }
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_owned(), value.into()));
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {token}"))
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub trait Transport: Send + Sync {
    /// Sends a request; any HTTP status is a successful `Ok`. Only network failures are errors.
    fn send(&self, request: &Request) -> CoreResult<Response>;

    /// GET streamed into `sink` (downloads can be large). Returns the HTTP status.
    fn download(&self, url: &str, headers: &[(String, String)], sink: &mut dyn Write) -> CoreResult<u16>;
}

fn is_allowed(url: &str) -> bool {
    url.strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

pub struct UreqTransport {
    agent: Agent,
}

impl UreqTransport {
    pub fn new() -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Self { agent }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

fn network_error(error: &ureq::Error) -> CoreError {
    CoreError::External(format!("network error: {error}"))
}

impl Transport for UreqTransport {
    fn send(&self, request: &Request) -> CoreResult<Response> {
        if !is_allowed(&request.url) {
            return Err(CoreError::External("the address is not an allowed Google host".into()));
        }
        let apply = |builder: ureq::RequestBuilder<ureq::typestate::WithBody>| {
            request.headers.iter().fold(builder, |builder, (name, value)| builder.header(name, value))
        };
        let apply_bodyless = |builder: ureq::RequestBuilder<ureq::typestate::WithoutBody>| {
            request.headers.iter().fold(builder, |builder, (name, value)| builder.header(name, value))
        };
        let result = match request.method {
            Method::Get => apply_bodyless(self.agent.get(&request.url)).call(),
            Method::Delete => apply_bodyless(self.agent.delete(&request.url)).call(),
            Method::Post => apply(self.agent.post(&request.url)).send(&request.body[..]),
            Method::Put => apply(self.agent.put(&request.url)).send(&request.body[..]),
            Method::Patch => apply(self.agent.patch(&request.url)).send(&request.body[..]),
        };
        let mut response = result.map_err(|error| network_error(&error))?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| value.to_str().ok().map(|value| (name.as_str().to_owned(), value.to_owned())))
            .collect();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_vec()
            .map_err(|error| network_error(&error))?;
        Ok(Response { status, headers, body })
    }

    fn download(&self, url: &str, headers: &[(String, String)], sink: &mut dyn Write) -> CoreResult<u16> {
        if !is_allowed(url) {
            return Err(CoreError::External("the address is not an allowed Google host".into()));
        }
        let builder = headers.iter().fold(self.agent.get(url), |builder, (name, value)| builder.header(name, value));
        let mut response = builder.call().map_err(|error| network_error(&error))?;
        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            std::io::copy(&mut response.body_mut().as_reader(), sink)?;
        }
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_google_hosts_over_https_are_reachable() {
        assert!(is_allowed("https://www.googleapis.com/drive/v3/files"));
        assert!(is_allowed("https://oauth2.googleapis.com/token"));
        assert!(!is_allowed("http://www.googleapis.com/drive/v3/files"));
        assert!(!is_allowed("https://www.googleapis.com.evil.example/x"));
        assert!(!is_allowed("https://evil.example/www.googleapis.com"));
    }

    #[test]
    fn response_headers_are_case_insensitive() {
        let response = Response { status: 308, headers: vec![("Range".into(), "bytes=0-9".into())], body: Vec::new() };
        assert_eq!(response.header("range"), Some("bytes=0-9"));
        assert!(!response.is_success());
    }
}
