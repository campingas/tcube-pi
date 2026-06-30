use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use crate::config::AdminConfig;

use super::handler::{self, error_response, json_response, HttpRequest, HttpResponse};

pub mod auth;
pub mod content;
pub mod multipart;
pub mod provider;
pub mod setup;

type AdminState = Arc<AdminConfig>;

pub(crate) fn router() -> Router<AdminState> {
    Router::new()
        .route("/api/pi/v1/status", get(status))
        .route("/api/auth/session", get(auth_session))
        .route("/api/pi/v1/auth/session", get(auth_session))
        .route("/api/auth/login/password", post(login_password))
        .route("/api/pi/v1/auth/login/password", post(login_password))
        .route("/api/auth/bootstrap", post(bootstrap_owner))
        .route("/api/pi/v1/auth/bootstrap", post(bootstrap_owner))
        .route("/api/auth/recover", post(recover_password))
        .route("/api/pi/v1/auth/recover", post(recover_password))
        .route("/api/auth/recovery-code", post(create_recovery_code))
        .route("/api/pi/v1/auth/recovery-code", post(create_recovery_code))
        .route("/api/auth/invitations", post(create_invitation))
        .route("/api/pi/v1/auth/invitations", post(create_invitation))
        .route("/api/auth/invitations/accept", post(accept_invitation))
        .route(
            "/api/pi/v1/auth/invitations/accept",
            post(accept_invitation),
        )
        .route("/api/auth/logout", post(logout))
        .route("/api/pi/v1/auth/logout", post(logout))
        .route("/api/setup/review", get(setup_review))
        .route("/api/pi/v1/setup/review", get(setup_review))
        .route("/api/setup/name", post(set_cube_name))
        .route("/api/pi/v1/setup/name", post(set_cube_name))
        .route("/api/setup/wifi/verified", post(verify_wifi))
        .route("/api/pi/v1/setup/wifi/verified", post(verify_wifi))
        .route("/api/setup/complete", post(complete_setup))
        .route("/api/pi/v1/setup/complete", post(complete_setup))
        .route("/api/setup/factory-reset", post(factory_reset))
        .route("/api/pi/v1/setup/factory-reset", post(factory_reset))
        .route("/api/setup/buttons/{button_id}/mode", post(set_button_mode))
        .route(
            "/api/pi/v1/setup/buttons/{button_id}/mode",
            post(set_button_mode),
        )
        .route("/api/content/recordings", post(save_recording))
        .route("/api/pi/v1/content/recordings", post(save_recording))
        .route("/api/content/uploads", post(save_upload))
        .route("/api/pi/v1/content/uploads", post(save_upload))
        .route("/api/content/generated-speech", post(save_generated_speech))
        .route(
            "/api/pi/v1/content/generated-speech",
            post(save_generated_speech),
        )
        .route(
            "/api/content/generated-speech/status",
            get(generated_speech_status),
        )
        .route(
            "/api/pi/v1/content/generated-speech/status",
            get(generated_speech_status),
        )
        .route("/api/content/inventory", get(content_inventory))
        .route("/api/pi/v1/content/inventory", get(content_inventory))
        .route(
            "/api/content/buttons/{button_id}/{content_type}/active",
            get(list_active_content),
        )
        .route(
            "/api/pi/v1/content/buttons/{button_id}/{content_type}/active",
            get(list_active_content),
        )
        .route(
            "/api/content/buttons/{button_id}/{content_type}/inactive",
            get(list_inactive_content),
        )
        .route(
            "/api/pi/v1/content/buttons/{button_id}/{content_type}/inactive",
            get(list_inactive_content),
        )
        .route(
            "/api/content/items/{item_id}/activate",
            post(activate_content_item),
        )
        .route(
            "/api/pi/v1/content/items/{item_id}/activate",
            post(activate_content_item),
        )
        .route(
            "/api/content/generated-speech/unused",
            delete(trash_unused_generated_speech),
        )
        .route(
            "/api/pi/v1/content/generated-speech/unused",
            delete(trash_unused_generated_speech),
        )
        .route("/api/content/unused", delete(trash_unused_content))
        .route("/api/pi/v1/content/unused", delete(trash_unused_content))
        .route("/api/content/items/{item_id}", delete(trash_content_item))
        .route(
            "/api/pi/v1/content/items/{item_id}",
            delete(trash_content_item),
        )
        .route("/api/events/recent", get(recent_button_events))
        .route("/api/pi/v1/events/recent", get(recent_button_events))
        .route("/api/media/{*path}", get(serve_media))
        .route("/api/pi/v1/media/{*path}", get(serve_media))
        .route("/media/{*path}", get(serve_media))
        .route("/content/{*path}", get(serve_content))
        .fallback(get(serve_static))
}

async fn status(State(config): State<AdminState>) -> Response {
    json_response(200, auth::pi_status(&config)).into_response()
}

async fn auth_session(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(config, request, |config, request| match auth::auth_session(
        config,
        request.session_cookie(),
    ) {
        Ok((body, cookie)) => {
            let mut response = json_response(200, body);
            if let Some(cookie) = cookie {
                response.headers.push(("Set-Cookie".to_string(), cookie));
            }
            response
        }
        Err(error) => error_response(500, error.to_string()),
    })
    .await
}

async fn login_password(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::login_password(config, request) {
            Ok((body, cookie)) => {
                let mut response = json_response(200, body);
                response.headers.push(("Set-Cookie".to_string(), cookie));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn bootstrap_owner(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::bootstrap_owner(config, request) {
            Ok((body, cookie)) => {
                let mut response = json_response(200, body);
                response.headers.push(("Set-Cookie".to_string(), cookie));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn recover_password(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::recover_password(config, request) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn create_recovery_code(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::create_recovery_code(config, request.session_cookie()) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(401, error.to_string()),
        },
    )
    .await
}

async fn create_invitation(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::create_invitation(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn accept_invitation(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match auth::accept_invitation(config, request) {
            Ok((body, cookie)) => {
                let mut response = json_response(200, body);
                response.headers.push(("Set-Cookie".to_string(), cookie));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn logout(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(config, request, |config, request| {
        match auth::logout(config, request.session_cookie()) {
            Ok(()) => {
                let mut response = json_response(200, serde_json::json!({ "status": "ok" }));
                response
                    .headers
                    .push(("Set-Cookie".to_string(), auth::clear_session_cookie()));
                response
            }
            Err(error) => error_response(500, error.to_string()),
        }
    })
    .await
}

async fn setup_review(State(config): State<AdminState>) -> Response {
    response_from_result(setup::setup_review(&config), 500)
}

async fn set_cube_name(State(config): State<AdminState>, request: Request<Body>) -> Response {
    response_from_request_result(config, request, setup::set_cube_name).await
}

async fn verify_wifi(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match setup::verify_wifi(config, request) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn set_button_mode(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match setup::set_button_mode(config, request, &request.path) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn complete_setup(State(config): State<AdminState>, request: Request<Body>) -> Response {
    response_from_request_result(config, request, setup::complete_setup).await
}

async fn factory_reset(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match setup::factory_reset(config, request) {
            Ok(body) => {
                let mut response = json_response(200, body);
                response
                    .headers
                    .push(("Set-Cookie".to_string(), auth::clear_session_cookie()));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn save_recording(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match multipart::save_multipart_media(config, request, "recorded") {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn save_upload(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match multipart::save_multipart_media(config, request, "uploaded") {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn save_generated_speech(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    response_from_request_result(config, request, provider::save_generated_speech).await
}

async fn generated_speech_status(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    response_from_request_result(config, request, provider::generated_speech_status).await
}

async fn content_inventory(State(config): State<AdminState>, request: Request<Body>) -> Response {
    response_from_request_result(config, request, content::content_inventory).await
}

async fn list_active_content(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match content::list_active_content(config, request, &request.path) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn list_inactive_content(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    run_request(
        config,
        request,
        |config, request| match content::list_inactive_content(config, request, &request.path) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn activate_content_item(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    run_request(
        config,
        request,
        |config, request| match content::activate_content_item(config, request, &request.path) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn trash_unused_generated_speech(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    response_from_request_result(config, request, content::trash_unused_generated_speech).await
}

async fn trash_unused_content(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    response_from_request_result(config, request, content::trash_unused_content).await
}

async fn trash_content_item(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(
        config,
        request,
        |config, request| match content::trash_content_item(config, request, &request.path) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
    )
    .await
}

async fn recent_button_events(
    State(config): State<AdminState>,
    request: Request<Body>,
) -> Response {
    response_from_request_result(config, request, handler::recent_button_events).await
}

async fn serve_media(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(config, request, |config, request| {
        let Some(relative) = request
            .path
            .strip_prefix("/api/media/")
            .or_else(|| request.path.strip_prefix("/media/"))
        else {
            return error_response(404, "not found");
        };
        super::pages::serve_file(&config.media_root, relative)
    })
    .await
}

async fn serve_content(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(config, request, |config, request| {
        let Some(relative) = request.path.strip_prefix("/content/") else {
            return error_response(404, "not found");
        };
        super::pages::serve_file(&config.content_root, relative)
    })
    .await
}

async fn serve_static(State(config): State<AdminState>, request: Request<Body>) -> Response {
    run_request(config, request, |config, request| {
        super::pages::serve_static(&config.ui_dist, &request.path)
    })
    .await
}

async fn response_from_request_result<T>(
    config: AdminState,
    request: Request<Body>,
    operation: fn(&AdminConfig, &HttpRequest) -> Result<T>,
) -> Response
where
    T: serde::Serialize + 'static,
{
    run_request(config, request, move |config, request| {
        http_response_from_result(operation(config, request), 400)
    })
    .await
}

fn response_from_result<T>(result: Result<T>, error_status: u16) -> Response
where
    T: serde::Serialize,
{
    http_response_from_result(result, error_status).into_response()
}

fn http_response_from_result<T>(result: Result<T>, error_status: u16) -> HttpResponse
where
    T: serde::Serialize,
{
    match result {
        Ok(body) => json_response(200, body),
        Err(error) => error_response(error_status, error.to_string()),
    }
}

async fn run_request(
    config: AdminState,
    request: Request<Body>,
    operation: impl FnOnce(&AdminConfig, &HttpRequest) -> HttpResponse + Send + 'static,
) -> Response {
    let mut request = match HttpRequest::from_request(request).await {
        Ok(request) => request,
        Err(error) => return error_response(400, error.to_string()).into_response(),
    };
    request.path = canonical_admin_api_path(&request.path);
    match tokio::task::spawn_blocking(move || operation(config.as_ref(), &request)).await {
        Ok(response) => response.into_response(),
        Err(error) => error_response(500, format!("admin request failed: {error}")).into_response(),
    }
}

#[cfg(test)]
pub(crate) fn route_request(request: &HttpRequest, config: &AdminConfig) -> HttpResponse {
    let path = canonical_admin_api_path(&request.path);

    match (request.method.as_str(), path.as_str()) {
        ("GET", "/api/pi/v1/status") => json_response(200, auth::pi_status(config)),
        ("GET", "/api/auth/session") => {
            match auth::auth_session(config, request.session_cookie()) {
                Ok((body, cookie)) => {
                    let mut response = json_response(200, body);
                    if let Some(cookie) = cookie {
                        response.headers.push(("Set-Cookie".to_string(), cookie));
                    }
                    response
                }
                Err(error) => error_response(500, error.to_string()),
            }
        }
        ("POST", "/api/auth/login/password") => match auth::login_password(config, request) {
            Ok((body, cookie)) => {
                let mut response = json_response(200, body);
                response.headers.push(("Set-Cookie".to_string(), cookie));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/auth/bootstrap") => match auth::bootstrap_owner(config, request) {
            Ok((body, cookie)) => {
                let mut response = json_response(200, body);
                response.headers.push(("Set-Cookie".to_string(), cookie));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/auth/recover") => match auth::recover_password(config, request) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/auth/recovery-code") => {
            match auth::create_recovery_code(config, request.session_cookie()) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(401, error.to_string()),
            }
        }
        ("POST", "/api/auth/invitations") => match auth::create_invitation(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/auth/invitations/accept") => {
            match auth::accept_invitation(config, request) {
                Ok((body, cookie)) => {
                    let mut response = json_response(200, body);
                    response.headers.push(("Set-Cookie".to_string(), cookie));
                    response
                }
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("POST", "/api/auth/logout") => match auth::logout(config, request.session_cookie()) {
            Ok(()) => {
                let mut response = json_response(200, serde_json::json!({ "status": "ok" }));
                response
                    .headers
                    .push(("Set-Cookie".to_string(), auth::clear_session_cookie()));
                response
            }
            Err(error) => error_response(500, error.to_string()),
        },
        ("POST", "/api/setup/name") => match setup::set_cube_name(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/setup/wifi/verified") => match setup::verify_wifi(config, request) {
            Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/setup/complete") => match setup::complete_setup(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", "/api/setup/factory-reset") => match setup::factory_reset(config, request) {
            Ok(body) => {
                let mut response = json_response(200, body);
                response
                    .headers
                    .push(("Set-Cookie".to_string(), auth::clear_session_cookie()));
                response
            }
            Err(error) => error_response(400, error.to_string()),
        },
        ("POST", path) if path.starts_with("/api/setup/buttons/") && path.ends_with("/mode") => {
            match setup::set_button_mode(config, request, path) {
                Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("POST", "/api/content/recordings") => {
            match multipart::save_multipart_media(config, request, "recorded") {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("POST", "/api/content/uploads") => {
            match multipart::save_multipart_media(config, request, "uploaded") {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("POST", "/api/content/generated-speech") => {
            match provider::save_generated_speech(config, request) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("GET", "/api/content/generated-speech/status") => {
            match provider::generated_speech_status(config, request) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("GET", "/api/content/inventory") => match content::content_inventory(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
        ("GET", path) if path.starts_with("/api/content/buttons/") && path.ends_with("/active") => {
            match content::list_active_content(config, request, path) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("GET", path)
            if path.starts_with("/api/content/buttons/") && path.ends_with("/inactive") =>
        {
            match content::list_inactive_content(config, request, path) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("POST", path)
            if path.starts_with("/api/content/items/") && path.ends_with("/activate") =>
        {
            match content::activate_content_item(config, request, path) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("DELETE", "/api/content/generated-speech/unused") => {
            match content::trash_unused_generated_speech(config, request) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("DELETE", "/api/content/unused") => match content::trash_unused_content(config, request) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(400, error.to_string()),
        },
        ("DELETE", path) if path.starts_with("/api/content/items/") => {
            match content::trash_content_item(config, request, path) {
                Ok(()) => json_response(200, serde_json::json!({ "status": "ok" })),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("GET", "/api/setup/review") => match setup::setup_review(config) {
            Ok(body) => json_response(200, body),
            Err(error) => error_response(500, error.to_string()),
        },
        ("GET", "/api/events/recent") => {
            match super::handler::recent_button_events(config, request) {
                Ok(body) => json_response(200, body),
                Err(error) => error_response(400, error.to_string()),
            }
        }
        ("GET", path) if path.starts_with("/api/media/") => {
            super::pages::serve_file(&config.media_root, path.trim_start_matches("/api/media/"))
        }
        ("GET", path) if path.starts_with("/media/") => {
            super::pages::serve_file(&config.media_root, path.trim_start_matches("/media/"))
        }
        ("GET", path) if path.starts_with("/content/") => {
            super::pages::serve_file(&config.content_root, path.trim_start_matches("/content/"))
        }
        ("GET", _) => super::pages::serve_static(&config.ui_dist, &request.path),
        _ => error_response(405, "method not allowed"),
    }
}

fn canonical_admin_api_path(path: &str) -> String {
    if path == "/api/pi/v1/status" {
        return path.to_string();
    }

    for prefix in ["/auth/", "/setup/", "/content/", "/media/", "/events/"] {
        let versioned_prefix = format!("/api/pi/v1{prefix}");
        if let Some(rest) = path.strip_prefix(&versioned_prefix) {
            return format!("/api{prefix}{rest}");
        }
    }

    path.to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use crate::config::AdminConfig;

    fn test_config(root: &TempDir) -> AdminConfig {
        AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: root.path().join("tcube.sqlite3"),
            ui_dist: root.path().join("admin-ui"),
            media_root: root.path().join("audio"),
            content_root: PathBuf::from("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
        }
    }

    #[tokio::test]
    async fn explicit_router_serves_versioned_status_and_bootstrap() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));

        let status = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);

        let bootstrap = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/pi/v1/auth/bootstrap")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "username": "parent",
                            "display_name": "Parent",
                            "password": "owner-password"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bootstrap.status(), StatusCode::OK);
        assert!(bootstrap.headers().contains_key("set-cookie"));
        let body = to_bytes(bootstrap.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["authenticated"], true);
        assert_eq!(body["cubes"][0]["role"], "owner");
    }
}
