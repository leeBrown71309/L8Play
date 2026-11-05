//! `l8art://` protocol: serves cached artwork to the WebView.
//!
//! A custom protocol replaces Tauri's generic asset protocol, whose scope matching works on
//! canonical file paths and breaks when the file system redirects them. Requests are mapped to
//! files by [`l8_core::artwork::resolve_public_path`], which accepts only the cache's own file names.

use std::borrow::Cow;

use l8_core::artwork::resolve_public_path;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

fn status(code: StatusCode) -> Response<Cow<'static, [u8]>> {
    Response::builder().status(code).body(Cow::Borrowed(&[][..])).unwrap_or_default()
}

pub fn respond(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let state = app.state::<AppState>();
    let Some((path, content_type)) = resolve_public_path(state.artwork_root(), request.uri().path()) else {
        return status(StatusCode::NOT_FOUND);
    };
    match std::fs::read(&path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            // File names contain a content hash, so a cached copy is never stale.
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(Cow::Owned(bytes))
            .unwrap_or_default(),
        Err(_) => status(StatusCode::NOT_FOUND),
    }
}
