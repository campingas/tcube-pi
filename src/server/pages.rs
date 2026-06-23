use std::fs;
use std::path::{Component, Path};

use super::handler::{error_response, HttpResponse};

pub(crate) fn serve_static(root: &Path, request_path: &str) -> HttpResponse {
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

pub(crate) fn serve_file(root: &Path, relative: &str) -> HttpResponse {
    if !is_safe_relative_path(relative) {
        return error_response(400, "invalid path");
    }
    serve_path(&root.join(relative))
}

fn serve_path(path: &Path) -> HttpResponse {
    match fs::read(path) {
        Ok(body) => HttpResponse {
            status: 200,
            content_type: content_type(path),
            headers: Vec::new(),
            body,
        },
        Err(_) => error_response(404, "not found"),
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
