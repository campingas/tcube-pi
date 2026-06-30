use std::path::{Component, Path};

use axum::body::Body;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

pub(crate) async fn serve_static(root: &Path, request_path: &str) -> Response {
    let relative = request_path.trim_start_matches('/');
    let (request_path, content_path) = if is_safe_relative_path(relative) {
        (request_path, static_content_path(root, relative))
    } else {
        ("/", root.join("index.html"))
    };

    let service = ServeDir::new(root).fallback(ServeFile::new(root.join("index.html")));
    serve_with(service, request_path, &content_path).await
}

pub(crate) async fn serve_file(root: &Path, relative: &str) -> Response {
    if !is_safe_relative_path(relative) {
        return error_response(StatusCode::BAD_REQUEST, "invalid path");
    }
    let service = ServeDir::new(root);
    let request_path = format!("/{relative}");
    let response = serve_with(service, &request_path, Path::new(relative)).await;
    if response.status() == StatusCode::NOT_FOUND {
        error_response(StatusCode::NOT_FOUND, "not found")
    } else {
        response
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

fn static_content_path(root: &Path, relative: &str) -> std::path::PathBuf {
    if relative.is_empty() || Path::new(relative).extension().is_none() {
        root.join("index.html")
    } else {
        root.join(relative)
    }
}

async fn serve_with<S>(service: S, request_path: &str, content_path: &Path) -> Response
where
    S: tower::Service<Request<Body>, Error = std::convert::Infallible>,
    S::Response: axum::response::IntoResponse,
    S::Future: Send + 'static,
{
    let request = Request::builder()
        .uri(request_path)
        .body(Body::empty())
        .expect("static file request should be valid");
    let mut response = service
        .oneshot(request)
        .await
        .expect("static file service should be infallible")
        .into_response();
    if response.status() == StatusCode::OK {
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(content_type(content_path)),
        );
        response
            .headers_mut()
            .insert(CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
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
    use std::fs;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validates_relative_paths() {
        assert!(is_safe_relative_path("assets/index.js"));
        assert!(!is_safe_relative_path("../data/tcube.sqlite3"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }

    #[tokio::test]
    async fn serves_html_for_root_fallback() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("index.html"), "<!doctype html>").unwrap();

        let response = serve_static(root.path(), "/").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            response.headers().get(CONTENT_DISPOSITION).unwrap(),
            "inline"
        );
    }

    #[tokio::test]
    async fn serves_html_for_extensionless_spa_fallback() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("index.html"), "<!doctype html>").unwrap();

        let response = serve_static(root.path(), "/settings").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            response.headers().get(CONTENT_DISPOSITION).unwrap(),
            "inline"
        );
    }

    #[tokio::test]
    async fn serves_html_for_rejected_static_paths() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("index.html"), "<!doctype html>").unwrap();

        let response = serve_static(root.path(), "/../tcube.sqlite3").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }
}
