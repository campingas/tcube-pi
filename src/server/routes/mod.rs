use std::sync::Arc;

use anyhow::Result;
use axum::extract::{DefaultBodyLimit, Multipart, OriginalUri, Path, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::config::AdminConfig;

use super::handler;
use super::media::{media_input_from_axum_multipart, MAX_AUDIO_BYTES};

pub mod auth;
pub mod content;
pub mod error;
pub mod setup;

type AdminState = Arc<AdminConfig>;

use error::{ApiError, SessionCookie};

#[derive(Debug, Deserialize)]
struct ContentQuery {
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratedSpeechStatusQuery {
    language: Option<String>,
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct OkResponse {
    status: &'static str,
}

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
        .layer(DefaultBodyLimit::max(MAX_AUDIO_BYTES + 1024 * 1024))
}

async fn status(State(config): State<AdminState>) -> Json<handler::StatusResponse> {
    Json(handler::pi_status(&config))
}

async fn auth_session(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Response, ApiError> {
    let (body, cookie) = blocking(config, move |config| {
        auth::auth_session(config, token.as_deref())
    })
    .await
    .map_err(ApiError::server)?;
    Ok(match cookie {
        Some(cookie) => json_with_cookie(body, cookie),
        None => Json(body).into_response(),
    })
}

async fn login_password(
    State(config): State<AdminState>,
    Json(body): Json<auth::LoginRequest>,
) -> Result<Response, ApiError> {
    let (body, cookie) = blocking(config, move |config| auth::login_password(config, body))
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json_with_cookie(body, cookie))
}

async fn bootstrap_owner(
    State(config): State<AdminState>,
    Json(body): Json<auth::BootstrapRequest>,
) -> Result<Response, ApiError> {
    let (body, cookie) = blocking(config, move |config| auth::bootstrap_owner(config, body))
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json_with_cookie(body, cookie))
}

async fn recover_password(
    State(config): State<AdminState>,
    Json(body): Json<auth::RecoverRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    blocking(config, move |config| auth::recover_password(config, body))
        .await
        .map_err(ApiError::bad_request)?;
    Ok(ok_json())
}

async fn create_recovery_code(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<auth::RecoveryCodeResponse>, ApiError> {
    blocking(config, move |config| {
        auth::create_recovery_code(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::unauthorized)
}

async fn create_invitation(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<auth::InvitationCreateRequest>,
) -> Result<Json<auth::InvitationResponse>, ApiError> {
    blocking(config, move |config| {
        auth::create_invitation(config, token.as_deref(), body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn accept_invitation(
    State(config): State<AdminState>,
    Json(body): Json<auth::InvitationAcceptRequest>,
) -> Result<Response, ApiError> {
    let (body, cookie) = blocking(config, move |config| auth::accept_invitation(config, body))
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json_with_cookie(body, cookie))
}

async fn logout(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Response, ApiError> {
    blocking(config, move |config| auth::logout(config, token.as_deref()))
        .await
        .map_err(ApiError::server)?;
    Ok(json_with_cookie(ok_body(), auth::clear_session_cookie()))
}

async fn setup_review(
    State(config): State<AdminState>,
) -> Result<Json<setup::SetupReviewResponse>, ApiError> {
    blocking(config, setup::setup_review)
        .await
        .map(Json)
        .map_err(ApiError::server)
}

async fn set_cube_name(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<setup::NameRequest>,
) -> Result<Json<setup::CubeSaveResponse>, ApiError> {
    blocking(config, move |config| {
        setup::set_cube_name(config, token.as_deref(), body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn verify_wifi(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<setup::WifiRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    blocking(config, move |config| {
        setup::verify_wifi(config, token.as_deref(), body)
    })
    .await
    .map_err(ApiError::bad_request)?;
    Ok(ok_json())
}

async fn set_button_mode(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path(button_id): Path<i64>,
    Json(body): Json<setup::ButtonModeRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    blocking(config, move |config| {
        setup::set_button_mode(config, token.as_deref(), button_id, body)
    })
    .await
    .map_err(ApiError::bad_request)?;
    Ok(ok_json())
}

async fn complete_setup(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<setup::CompleteSetupResponse>, ApiError> {
    blocking(config, move |config| {
        setup::complete_setup(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn factory_reset(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<setup::FactoryResetRequest>,
) -> Result<Response, ApiError> {
    let body = blocking(config, move |config| {
        setup::factory_reset(config, token.as_deref(), body)
    })
    .await
    .map_err(ApiError::bad_request)?;
    Ok(json_with_cookie(body, auth::clear_session_cookie()))
}

async fn save_recording(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    multipart: Multipart,
) -> Result<Json<content::InactiveContentResponse>, ApiError> {
    let input = media_input_from_axum_multipart(multipart)
        .await
        .map_err(ApiError::bad_request)?;
    blocking(config, move |config| {
        handler::save_multipart_media(config, token.as_deref(), input, "recorded")
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn save_upload(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    multipart: Multipart,
) -> Result<Json<content::InactiveContentResponse>, ApiError> {
    let input = media_input_from_axum_multipart(multipart)
        .await
        .map_err(ApiError::bad_request)?;
    blocking(config, move |config| {
        handler::save_multipart_media(config, token.as_deref(), input, "uploaded")
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn save_generated_speech(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<handler::GeneratedSpeechRequest>,
) -> Result<Json<content::InactiveContentResponse>, ApiError> {
    blocking(config, move |config| {
        handler::save_generated_speech(config, token.as_deref(), body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn generated_speech_status(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Query(query): Query<GeneratedSpeechStatusQuery>,
) -> Result<Json<super::speech::GeneratedSpeechStatusResponse>, ApiError> {
    let language = query.language.unwrap_or_else(|| "English".to_string());
    let provider = query.provider.unwrap_or_else(|| "auto".to_string());
    blocking(config, move |config| {
        handler::generated_speech_status(config, token.as_deref(), &provider, &language)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn content_inventory(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<content::ContentInventoryResponse>, ApiError> {
    blocking(config, move |config| {
        content::content_inventory(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn list_active_content(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path((button_id, content_type)): Path<(i64, String)>,
    Query(query): Query<ContentQuery>,
) -> Result<Json<content::ContentListResponse<content::ActiveContentResponse>>, ApiError> {
    blocking(config, move |config| {
        content::list_active_content(
            config,
            token.as_deref(),
            button_id,
            &content_type,
            query.language.as_deref(),
        )
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn list_inactive_content(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path((button_id, content_type)): Path<(i64, String)>,
    Query(query): Query<ContentQuery>,
) -> Result<Json<content::ContentListResponse<content::InactiveContentResponse>>, ApiError> {
    blocking(config, move |config| {
        content::list_inactive_content(
            config,
            token.as_deref(),
            button_id,
            &content_type,
            query.language.as_deref(),
        )
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn activate_content_item(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path(item_id): Path<String>,
) -> Result<Json<content::InactiveContentResponse>, ApiError> {
    blocking(config, move |config| {
        content::activate_content_item(config, token.as_deref(), &item_id)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn trash_unused_generated_speech(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<content::GeneratedCleanupRequest>,
) -> Result<Json<content::CleanupResponse>, ApiError> {
    blocking(config, move |config| {
        content::trash_unused_generated_speech(config, token.as_deref(), body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn trash_unused_content(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<content::CleanupResponse>, ApiError> {
    blocking(config, move |config| {
        content::trash_unused_content(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn trash_content_item(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path(item_id): Path<String>,
) -> Result<Json<OkResponse>, ApiError> {
    blocking(config, move |config| {
        content::trash_content_item(config, token.as_deref(), &item_id)
    })
    .await
    .map_err(ApiError::bad_request)?;
    Ok(ok_json())
}

async fn recent_button_events(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<Vec<handler::RecentButtonEventResponse>>, ApiError> {
    blocking(config, move |config| {
        handler::recent_button_events(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn serve_media(State(config): State<AdminState>, Path(path): Path<String>) -> Response {
    super::pages::serve_file(&config.media_root, &path).await
}

async fn serve_content(State(config): State<AdminState>, Path(path): Path<String>) -> Response {
    super::pages::serve_file(&config.content_root, &path).await
}

async fn serve_static(State(config): State<AdminState>, OriginalUri(uri): OriginalUri) -> Response {
    super::pages::serve_static(&config.ui_dist, uri.path()).await
}

async fn blocking<T>(
    config: AdminState,
    operation: impl FnOnce(&AdminConfig) -> Result<T> + Send + 'static,
) -> Result<T, anyhow::Error>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || operation(config.as_ref()))
        .await
        .map_err(|error| anyhow::anyhow!("admin request failed: {error}"))?
}

fn ok_json() -> Json<OkResponse> {
    Json(ok_body())
}

fn ok_body() -> OkResponse {
    OkResponse { status: "ok" }
}

fn json_with_cookie<T: Serialize>(body: T, cookie: String) -> Response {
    let mut response = Json(body).into_response();
    if let Ok(value) = HeaderValue::try_from(cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
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
