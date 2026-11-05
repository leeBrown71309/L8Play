//! Google Drive API v3 client (`drive.file` scope): folders, metadata queries, resumable uploads
//! and streamed downloads. Layout in the user's Drive: `L8Play/<game slug>/<snapshot id>.zip`,
//! each archive carrying the snapshot's metadata in `appProperties` so a new PC can rebuild its
//! snapshot list from Drive alone.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::artwork::http::percent_encode;
use crate::cloud::oauth::TokenSource;
use crate::cloud::transport::{Method, Request, Response, Transport};
use crate::error::{CoreError, CoreResult};

const FILES: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const ABOUT: &str = "https://www.googleapis.com/drive/v3/about";
pub const ROOT_FOLDER_NAME: &str = "L8Play";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const ARCHIVE_MIME: &str = "application/zip";
/// Resumable chunks must be multiples of 256 KiB.
pub const CHUNK_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub description: Option<String>,
    pub parents: Vec<String>,
    pub app_properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct About {
    pub email: String,
    pub used: i64,
    /// `None` for unlimited (Workspace) accounts.
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UploadSession {
    pub resumable_uri: Option<String>,
    pub bytes_done: i64,
}

#[derive(Deserialize)]
struct FileJson {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default, rename = "appProperties")]
    app_properties: HashMap<String, String>,
}

impl From<FileJson> for DriveFile {
    fn from(file: FileJson) -> Self {
        Self {
            id: file.id,
            name: file.name,
            size: file.size.and_then(|size| size.parse().ok()).unwrap_or(0),
            description: file.description,
            parents: file.parents,
            app_properties: file.app_properties,
        }
    }
}

#[derive(Deserialize)]
struct ListJson {
    #[serde(default)]
    files: Vec<FileJson>,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
}

/// Everything needed to start (or resume) the upload of one archive.
pub struct UploadSpec<'a> {
    pub parent: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub app_properties: &'a HashMap<String, String>,
    pub path: &'a Path,
}

pub struct DriveClient<'a> {
    transport: &'a dyn Transport,
    tokens: &'a dyn TokenSource,
}

/// Escapes a value inside a single-quoted Drive query literal.
fn query_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

impl<'a> DriveClient<'a> {
    pub fn new(transport: &'a dyn Transport, tokens: &'a dyn TokenSource) -> Self {
        Self { transport, tokens }
    }

    /// Sends an authorized request; a 401 refreshes nothing here (the token source already
    /// refreshes before expiry), so it surfaces as an authentication error.
    fn call(&self, request: Request) -> CoreResult<Response> {
        let token = self.tokens.access_token()?;
        let response = self.transport.send(&request.bearer(&token))?;
        classify(response)
    }

    pub fn about(&self) -> CoreResult<About> {
        let response = self.call(Request::new(
            Method::Get,
            format!("{ABOUT}?fields={}", percent_encode("user(emailAddress),storageQuota(limit,usage)")),
        ))?;
        let json: serde_json::Value = parse_json(&response)?;
        let text = |value: &serde_json::Value| value.as_str().and_then(|s| s.parse::<i64>().ok());
        Ok(About {
            email: json["user"]["emailAddress"].as_str().unwrap_or_default().to_owned(),
            used: text(&json["storageQuota"]["usage"]).unwrap_or(0),
            limit: text(&json["storageQuota"]["limit"]),
        })
    }

    /// Finds a folder created by this app, or creates it.
    pub fn ensure_folder(&self, name: &str, parent: Option<&str>) -> CoreResult<String> {
        let mut query = format!("mimeType='{FOLDER_MIME}' and name='{}' and trashed=false", query_literal(name));
        if let Some(parent) = parent {
            query.push_str(&format!(" and '{}' in parents", query_literal(parent)));
        }
        if let Some(existing) = self.list(&query)?.into_iter().next() {
            return Ok(existing.id);
        }
        let mut body = json!({ "name": name, "mimeType": FOLDER_MIME });
        if let Some(parent) = parent {
            body["parents"] = json!([parent]);
        }
        let response = self.call(
            Request::new(Method::Post, format!("{FILES}?fields=id"))
                .header("Content-Type", "application/json; charset=UTF-8")
                .body(body.to_string().into_bytes()),
        )?;
        let created: FileJson = parse_json(&response)?;
        Ok(created.id)
    }

    /// Every page of a `files.list` query, with the fields the sync needs.
    pub fn list(&self, query: &str) -> CoreResult<Vec<DriveFile>> {
        let mut files = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let mut url = format!(
                "{FILES}?q={}&pageSize=200&spaces=drive&fields={}",
                percent_encode(query),
                percent_encode("nextPageToken,files(id,name,size,description,parents,appProperties)")
            );
            if let Some(token) = &page {
                url.push_str(&format!("&pageToken={}", percent_encode(token)));
            }
            let response = self.call(Request::new(Method::Get, url))?;
            let parsed: ListJson = parse_json(&response)?;
            files.extend(parsed.files.into_iter().map(DriveFile::from));
            match parsed.next_page_token {
                Some(token) => page = Some(token),
                None => return Ok(files),
            }
        }
    }

    /// Permanently deletes one snapshot archive. Returns `false` when it was already gone.
    ///
    /// Safety: with the `drive.file` scope the app only sees files it created, and on top of that
    /// the file must carry this snapshot's `l8id` property, so nothing else can be removed even if a
    /// stored id were wrong.
    pub fn delete_snapshot_file(&self, file_id: &str, snapshot_id: &str) -> CoreResult<bool> {
        let id = percent_encode(file_id);
        let fields = percent_encode("id,appProperties");
        let file: FileJson = match self.call(Request::new(Method::Get, format!("{FILES}/{id}?fields={fields}"))) {
            Ok(response) => parse_json(&response)?,
            Err(CoreError::NotFound(_)) => return Ok(false),
            Err(error) => return Err(error),
        };
        if file.app_properties.get("l8id").map(String::as_str) != Some(snapshot_id) {
            return Err(CoreError::Validation(
                "the Drive file is not the snapshot it was recorded as; it was left alone".into(),
            ));
        }
        match self.call(Request::new(Method::Delete, format!("{FILES}/{id}"))) {
            Ok(_) => Ok(true),
            Err(CoreError::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Uploads a file with a resumable session, calling `on_progress` after each chunk so an
    /// interrupted upload can continue where it stopped.
    pub fn upload_resumable(
        &self,
        spec: &UploadSpec<'_>,
        session: &mut UploadSession,
        on_progress: &mut dyn FnMut(&UploadSession),
    ) -> CoreResult<DriveFile> {
        let path = spec.path;
        let total = std::fs::metadata(path)?.len();
        if session.resumable_uri.is_some() {
            match self.resume_offset(session, total)? {
                ResumeState::Complete(file) => return Ok(file),
                ResumeState::At(offset) => session.bytes_done = i64::try_from(offset).unwrap_or(0),
                ResumeState::Expired => *session = UploadSession::default(),
            }
        }
        if session.resumable_uri.is_none() {
            session.resumable_uri = Some(self.start_session(spec, total)?);
            session.bytes_done = 0;
            on_progress(session);
        }
        let uri = session.resumable_uri.clone().unwrap_or_default();
        let mut file = File::open(path)?;
        loop {
            let start = u64::try_from(session.bytes_done).unwrap_or(0);
            let length = CHUNK_BYTES.min(total - start);
            let mut chunk = vec![0u8; usize::try_from(length).unwrap_or(0)];
            file.seek(SeekFrom::Start(start))?;
            file.read_exact(&mut chunk)?;
            let end = start + length - 1;
            let token = self.tokens.access_token()?;
            let response = self.transport.send(
                &Request::new(Method::Put, uri.clone())
                    .bearer(&token)
                    .header("Content-Range", format!("bytes {start}-{end}/{total}"))
                    .body(chunk),
            )?;
            match response.status {
                200 | 201 => return Ok(parse_json::<FileJson>(&response)?.into()),
                308 => {
                    session.bytes_done = response
                        .header("Range")
                        .and_then(range_end)
                        .map_or(i64::try_from(end + 1).unwrap_or(0), |last| i64::try_from(last + 1).unwrap_or(0));
                    on_progress(session);
                }
                404 | 410 => {
                    *session = UploadSession::default();
                    return Err(CoreError::External("the upload session expired; it will restart".into()));
                }
                other => {
                    return Err(classify(response)
                        .err()
                        .unwrap_or_else(|| CoreError::External(format!("Drive answered HTTP {other} to a chunk"))));
                }
            }
        }
    }

    fn start_session(&self, spec: &UploadSpec<'_>, total: u64) -> CoreResult<String> {
        let mut metadata = json!({
            "name": spec.name,
            "parents": [spec.parent],
            "mimeType": ARCHIVE_MIME,
            "appProperties": spec.app_properties,
        });
        if let Some(description) = spec.description {
            metadata["description"] = json!(description);
        }
        let response = self.call(
            Request::new(
                Method::Post,
                format!("{UPLOAD}?uploadType=resumable&fields=id,name,size,parents,appProperties"),
            )
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("X-Upload-Content-Type", ARCHIVE_MIME)
            .header("X-Upload-Content-Length", total.to_string())
            .body(metadata.to_string().into_bytes()),
        )?;
        response
            .header("Location")
            .map(str::to_owned)
            .ok_or_else(|| CoreError::External("Drive did not return an upload address".into()))
    }

    /// Asks Drive how much of an interrupted upload it already has.
    fn resume_offset(&self, session: &UploadSession, total: u64) -> CoreResult<ResumeState> {
        let uri = session.resumable_uri.clone().unwrap_or_default();
        let token = self.tokens.access_token()?;
        let response = self
            .transport
            .send(&Request::new(Method::Put, uri).bearer(&token).header("Content-Range", format!("bytes */{total}")))?;
        Ok(match response.status {
            200 | 201 => ResumeState::Complete(parse_json::<FileJson>(&response)?.into()),
            308 => ResumeState::At(response.header("Range").and_then(range_end).map_or(0, |last| last + 1)),
            404 | 410 => ResumeState::Expired,
            _ => {
                classify(response)?;
                ResumeState::Expired
            }
        })
    }

    /// Streams a file's content into `dest` (written through a `.partial` sibling by the caller).
    pub fn download(&self, file_id: &str, sink: &mut dyn std::io::Write) -> CoreResult<()> {
        let token = self.tokens.access_token()?;
        let status = self.transport.download(
            &format!("{FILES}/{}?alt=media", percent_encode(file_id)),
            &[("Authorization".to_owned(), format!("Bearer {token}"))],
            sink,
        )?;
        match status {
            200..=299 => Ok(()),
            401 | 403 => Err(CoreError::Auth("Drive refused the download".into())),
            404 => Err(CoreError::NotFound("the file is no longer in Drive".into())),
            429 => Err(CoreError::Busy("Drive is rate limiting requests".into())),
            other => Err(CoreError::External(format!("Drive answered HTTP {other} to the download"))),
        }
    }
}

enum ResumeState {
    Complete(DriveFile),
    At(u64),
    Expired,
}

/// The last byte index in `Range: bytes=0-N`.
fn range_end(header: &str) -> Option<u64> {
    header.rsplit('-').next()?.trim().parse().ok()
}

fn classify(response: Response) -> CoreResult<Response> {
    match response.status {
        200..=299 => Ok(response),
        401 => Err(CoreError::Auth("Drive refused the access token".into())),
        403 => {
            let body = response.text();
            if body.contains("rateLimitExceeded") || body.contains("userRateLimitExceeded") {
                Err(CoreError::Busy("Drive is rate limiting requests".into()))
            } else if body.contains("storageQuotaExceeded") {
                Err(CoreError::Validation("your Google Drive storage is full".into()))
            } else {
                Err(CoreError::Auth("Drive refused the request (permission)".into()))
            }
        }
        404 => Err(CoreError::NotFound("Drive could not find that item".into())),
        429 => Err(CoreError::Busy("Drive is rate limiting requests".into())),
        status if status >= 500 => Err(CoreError::External(format!("Drive is unavailable (HTTP {status})"))),
        status => Err(CoreError::External(format!("Drive answered HTTP {status}"))),
    }
}

fn parse_json<T: for<'de> Deserialize<'de>>(response: &Response) -> CoreResult<T> {
    serde_json::from_slice(&response.body)
        .map_err(|error| CoreError::External(format!("unexpected answer from Drive: {error}")))
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::Mutex;

    use super::*;

    struct Token;
    impl TokenSource for Token {
        fn access_token(&self) -> CoreResult<String> {
            Ok("tok".into())
        }
    }

    /// Replays canned responses and records requests.
    struct Script {
        responses: Mutex<Vec<Response>>,
        requests: Mutex<Vec<Request>>,
    }

    impl Script {
        fn new(responses: Vec<Response>) -> Self {
            Self { responses: Mutex::new(responses), requests: Mutex::new(Vec::new()) }
        }
    }

    impl Transport for Script {
        fn send(&self, request: &Request) -> CoreResult<Response> {
            self.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?.push(request.clone());
            Ok(self.responses.lock().map_err(|_| CoreError::LockPoisoned("test"))?.remove(0))
        }

        fn download(&self, _url: &str, _headers: &[(String, String)], sink: &mut dyn Write) -> CoreResult<u16> {
            sink.write_all(b"zipdata")?;
            Ok(200)
        }
    }

    fn json_response(status: u16, body: &str) -> Response {
        Response { status, headers: Vec::new(), body: body.as_bytes().to_vec() }
    }

    #[test]
    fn deleting_a_snapshot_file_checks_its_identity_first() -> CoreResult<()> {
        let script = Script::new(vec![
            json_response(200, r#"{"id":"f1","appProperties":{"l8id":"snap-1"}}"#),
            json_response(204, ""),
            json_response(404, ""),
            json_response(200, r#"{"id":"f2","appProperties":{"l8id":"someone-else"}}"#),
        ]);
        let client = DriveClient::new(&script, &Token);
        assert!(client.delete_snapshot_file("f1", "snap-1")?);
        assert!(!client.delete_snapshot_file("gone", "snap-1")?, "already gone is not an error");
        assert!(matches!(client.delete_snapshot_file("f2", "snap-1"), Err(CoreError::Validation(_))));
        let requests = script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?;
        assert_eq!(requests.len(), 4, "the mismatching file is never deleted");
        assert_eq!(requests[1].method, Method::Delete);
        Ok(())
    }

    #[test]
    fn query_literals_are_escaped() {
        assert_eq!(query_literal("it's a \\ test"), "it\\'s a \\\\ test");
    }

    #[test]
    fn about_reads_the_account_and_quota() -> CoreResult<()> {
        let script = Script::new(vec![json_response(
            200,
            r#"{"user":{"emailAddress":"a@b.c"},"storageQuota":{"limit":"16106127360","usage":"1024"}}"#,
        )]);
        let about = DriveClient::new(&script, &Token).about()?;
        assert_eq!(about, About { email: "a@b.c".into(), used: 1024, limit: Some(16_106_127_360) });
        assert_eq!(
            script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?[0].headers[0],
            ("Authorization".into(), "Bearer tok".into())
        );
        Ok(())
    }

    #[test]
    fn ensure_folder_reuses_or_creates() -> CoreResult<()> {
        let script = Script::new(vec![
            json_response(200, r#"{"files":[{"id":"f1","name":"L8Play"}]}"#),
            json_response(200, r#"{"files":[]}"#),
            json_response(200, r#"{"id":"new"}"#),
        ]);
        let client = DriveClient::new(&script, &Token);
        assert_eq!(client.ensure_folder("L8Play", None)?, "f1");
        assert_eq!(client.ensure_folder("sekiro", Some("f1"))?, "new");
        let requests = script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?;
        assert!(requests[1].url.contains("%27f1%27%20in%20parents"));
        assert_eq!(requests[2].method, Method::Post);
        assert!(String::from_utf8_lossy(&requests[2].body).contains("\"parents\":[\"f1\"]"));
        Ok(())
    }

    #[test]
    fn list_follows_pages() -> CoreResult<()> {
        let script = Script::new(vec![
            json_response(
                200,
                r#"{"files":[{"id":"1","size":"10","appProperties":{"l8id":"x"}}],"nextPageToken":"p2"}"#,
            ),
            json_response(200, r#"{"files":[{"id":"2"}]}"#),
        ]);
        let files = DriveClient::new(&script, &Token).list("trashed=false")?;
        assert_eq!(files.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), vec!["1", "2"]);
        assert_eq!(files[0].size, 10);
        assert_eq!(files[0].app_properties["l8id"], "x");
        assert!(script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?[1].url.contains("pageToken=p2"));
        Ok(())
    }

    #[test]
    fn a_resumable_upload_sends_ranges_and_reports_progress() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.zip");
        std::fs::write(&path, vec![7u8; 10])?;
        let start = Response {
            status: 200,
            headers: vec![("Location".into(), "https://www.googleapis.com/upload/session/1".into())],
            body: Vec::new(),
        };
        let done = json_response(200, r#"{"id":"remote","name":"a.zip","size":"10"}"#);
        let script = Script::new(vec![start, done]);
        let mut session = UploadSession::default();
        let mut progress = Vec::new();
        let properties = HashMap::from([("l8id".to_owned(), "snap".to_owned())]);
        let spec = UploadSpec {
            parent: "parent",
            name: "a.zip",
            description: Some("note"),
            app_properties: &properties,
            path: &path,
        };
        let file = DriveClient::new(&script, &Token)
            .upload_resumable(&spec, &mut session, &mut |s| progress.push(s.clone()))?;
        assert_eq!(file.id, "remote");
        let requests = script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?;
        assert!(requests[0].url.contains("uploadType=resumable"));
        let metadata = String::from_utf8_lossy(&requests[0].body).into_owned();
        assert!(metadata.contains("\"l8id\":\"snap\"") && metadata.contains("\"description\":\"note\""));
        assert!(requests[0].headers.contains(&("X-Upload-Content-Length".into(), "10".into())));
        assert!(requests[1].headers.contains(&("Content-Range".into(), "bytes 0-9/10".into())));
        assert_eq!(progress[0].resumable_uri.as_deref(), Some("https://www.googleapis.com/upload/session/1"));
        Ok(())
    }

    #[test]
    fn an_interrupted_upload_resumes_from_the_offset_drive_reports() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.zip");
        std::fs::write(&path, vec![7u8; 10])?;
        let status = Response { status: 308, headers: vec![("Range".into(), "bytes=0-3".into())], body: Vec::new() };
        let done = json_response(201, r#"{"id":"remote"}"#);
        let script = Script::new(vec![status, done]);
        let mut session =
            UploadSession { resumable_uri: Some("https://www.googleapis.com/upload/session/1".into()), bytes_done: 2 };
        let properties = HashMap::new();
        let spec =
            UploadSpec { parent: "parent", name: "a.zip", description: None, app_properties: &properties, path: &path };
        DriveClient::new(&script, &Token).upload_resumable(&spec, &mut session, &mut |_| {})?;
        let requests = script.requests.lock().map_err(|_| CoreError::LockPoisoned("test"))?;
        assert!(requests[0].headers.contains(&("Content-Range".into(), "bytes */10".into())));
        assert!(requests[1].headers.contains(&("Content-Range".into(), "bytes 4-9/10".into())));
        assert_eq!(requests[1].body.len(), 6);
        Ok(())
    }

    #[test]
    fn statuses_map_to_actionable_errors() {
        assert!(matches!(classify(json_response(401, "")), Err(CoreError::Auth(_))));
        assert!(matches!(classify(json_response(403, "rateLimitExceeded")), Err(CoreError::Busy(_))));
        assert!(matches!(classify(json_response(403, "storageQuotaExceeded")), Err(CoreError::Validation(_))));
        assert!(matches!(classify(json_response(403, "forbidden")), Err(CoreError::Auth(_))));
        assert!(matches!(classify(json_response(404, "")), Err(CoreError::NotFound(_))));
        assert!(matches!(classify(json_response(429, "")), Err(CoreError::Busy(_))));
        assert!(matches!(classify(json_response(503, "")), Err(CoreError::External(_))));
    }

    #[test]
    fn downloads_stream_into_the_sink() -> CoreResult<()> {
        let script = Script::new(Vec::new());
        let mut sink = Vec::new();
        DriveClient::new(&script, &Token).download("abc", &mut sink)?;
        assert_eq!(sink, b"zipdata");
        Ok(())
    }

    #[test]
    fn range_headers_parse() {
        assert_eq!(range_end("bytes=0-262143"), Some(262_143));
        assert_eq!(range_end("junk"), None);
    }
}
