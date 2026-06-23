use crate::config::AdminConfig;

use super::handler::{error_response, json_response, HttpRequest, HttpResponse};

pub mod auth;
pub mod content;
pub mod multipart;
pub mod provider;
pub mod setup;

pub(crate) fn route_request(request: &HttpRequest, config: &AdminConfig) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
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
