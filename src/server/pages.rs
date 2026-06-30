use std::fs;
use std::path::{Component, Path};

use axum::body::Body;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use serde_json::json;

pub(crate) fn serve_static(root: &Path, request_path: &str) -> Response {
    let relative = request_path.trim_start_matches('/');
    let candidate = if relative.is_empty() {
        root.join("index.html")
    } else {
        root.join(relative)
    };

    let file = if is_safe_relative_path(relative) && candidate.is_file() {
        candidate
    } else {
        root.join("index.html")
    };
    serve_path(&file)
}

pub(crate) fn serve_file(root: &Path, relative: &str) -> Response {
    if !is_safe_relative_path(relative) {
        return error_response(StatusCode::BAD_REQUEST, "invalid path");
    }
    serve_path(&root.join(relative))
}

fn serve_path(path: &Path) -> Response {
    match fs::read(path) {
        Ok(body) => file_response(path, body),
        Err(_) => error_response(StatusCode::NOT_FOUND, "not found"),
    }
}

pub(crate) fn is_safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("mp3") => "audio/mpeg",
        Some("svg") => "image/svg+xml",
        Some("wav") => "audio/wav",
        Some("webmanifest") => "application/manifest+json",
        _ => "application/octet-stream",
    }
}

fn file_response(path: &Path, body: Vec<u8>) -> Response {
    let body_len = body.len().to_string();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type(path)));
    if let Ok(value) = HeaderValue::try_from(body_len) {
        headers.insert(CONTENT_LENGTH, value);
    }
    response
}

fn error_response(status: StatusCode, detail: &'static str) -> Response {
    let body = serde_json::to_vec(&json!({ "detail": detail }))
        .expect("serializing JSON error response should not fail");
    let body_len = body.len().to_string();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::try_from(body_len) {
        headers.insert(CONTENT_LENGTH, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_relative_paths() {
        assert!(is_safe_relative_path("assets/index.js"));
        assert!(!is_safe_relative_path("../data/tcube.sqlite3"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }
}
