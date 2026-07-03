use std::sync::Arc;

use anyhow::Result;
use axum::extract::{DefaultBodyLimit, Multipart, OriginalUri, Path, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put, MethodRouter};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::config::AdminConfig;

use super::media::{media_input_from_axum_multipart, MAX_AUDIO_BYTES};

pub mod auth;
pub mod content;
pub mod error;
pub mod events;
pub mod setup;
pub mod status;

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
    // Versioned-only routes without a legacy alias.
    let mut router = Router::new()
        .route("/api/pi/v1/status", get(status))
        .route("/api/pi/v1/setup/pomodoro", get(pomodoro_settings))
        .route("/api/pi/v1/setup/pomodoro", put(save_pomodoro_settings));

    // API routes registered at both the legacy /api/... path and the versioned /api/pi/v1/... path.
    let dual_routes: Vec<(&str, MethodRouter<AdminState>)> = vec![
        ("/auth/session", get(auth_session)),
        ("/auth/login/password", post(login_password)),
        ("/auth/bootstrap", post(bootstrap_owner)),
        ("/auth/recover", post(recover_password)),
        ("/auth/recovery-code", post(create_recovery_code)),
        ("/auth/invitations", post(create_invitation)),
        ("/auth/invitations/accept", post(accept_invitation)),
        ("/auth/logout", post(logout)),
        ("/setup/review", get(setup_review)),
        ("/setup/name", post(set_cube_name)),
        ("/setup/wifi/verified", post(verify_wifi)),
        ("/setup/complete", post(complete_setup)),
        ("/setup/factory-reset", post(factory_reset)),
        ("/setup/buttons/{button_id}/mode", post(set_button_mode)),
        ("/content/recordings", post(save_recording)),
        ("/content/uploads", post(save_upload)),
        ("/content/generated-speech", post(save_generated_speech)),
        (
            "/content/generated-speech/status",
            get(generated_speech_status),
        ),
        ("/content/inventory", get(content_inventory)),
        (
            "/content/buttons/{button_id}/{content_type}/active",
            get(list_active_content),
        ),
        (
            "/content/buttons/{button_id}/{content_type}/inactive",
            get(list_inactive_content),
        ),
        (
            "/content/items/{item_id}/activate",
            post(activate_content_item),
        ),
        (
            "/content/buttons/{button_id}/soundbox",
            get(list_soundbox_catalog),
        ),
        (
            "/content/buttons/{button_id}/soundbox/{slug}",
            post(set_soundbox_selection),
        ),
        ("/content/soundbox/{slug}/preview", get(soundbox_preview)),
        (
            "/content/generated-speech/unused",
            delete(trash_unused_generated_speech),
        ),
        ("/content/unused", delete(trash_unused_content)),
        ("/content/items/{item_id}", delete(trash_content_item)),
        ("/events/recent", get(recent_button_events)),
        ("/media/{*path}", get(serve_media)),
    ];
    for (path, method_router) in dual_routes {
        router = router
            .route(&format!("/api{path}"), method_router.clone())
            .route(&format!("/api/pi/v1{path}"), method_router);
    }

    router
        .route("/media/{*path}", get(serve_media))
        .route("/content/{*path}", get(serve_content))
        .fallback(get(serve_static))
        .layer(DefaultBodyLimit::max(MAX_AUDIO_BYTES + 1024 * 1024))
}

async fn status(State(config): State<AdminState>) -> Json<status::StatusResponse> {
    Json(status::pi_status(&config))
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

async fn pomodoro_settings(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
) -> Result<Json<setup::PomodoroSettingsWithRecommendation>, ApiError> {
    blocking(config, move |config| {
        setup::pomodoro_settings(config, token.as_deref())
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn save_pomodoro_settings(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<setup::PomodoroSettingsUpdate>,
) -> Result<Json<setup::PomodoroSettingsWithRecommendation>, ApiError> {
    blocking(config, move |config| {
        setup::save_pomodoro_settings(config, token.as_deref(), body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
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
        content::save_multipart_media(config, token.as_deref(), input, "recorded")
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
        content::save_multipart_media(config, token.as_deref(), input, "uploaded")
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn save_generated_speech(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Json(body): Json<content::GeneratedSpeechRequest>,
) -> Result<Json<content::InactiveContentResponse>, ApiError> {
    blocking(config, move |config| {
        content::save_generated_speech(config, token.as_deref(), body)
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
        content::generated_speech_status(config, token.as_deref(), &provider, &language)
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

async fn list_soundbox_catalog(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path(button_id): Path<i64>,
) -> Result<Json<content::SoundboxCatalogResponse>, ApiError> {
    blocking(config, move |config| {
        content::list_soundbox_catalog(config, token.as_deref(), button_id)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn set_soundbox_selection(
    State(config): State<AdminState>,
    SessionCookie(token): SessionCookie,
    Path((button_id, slug)): Path<(i64, String)>,
    Json(body): Json<content::SoundboxSelectionRequest>,
) -> Result<Json<content::SoundboxCatalogResponse>, ApiError> {
    blocking(config, move |config| {
        content::set_soundbox_selection(config, token.as_deref(), button_id, &slug, body)
    })
    .await
    .map(Json)
    .map_err(ApiError::bad_request)
}

async fn soundbox_preview(Path(slug): Path<String>) -> Result<Response, ApiError> {
    let wav = tokio::task::spawn_blocking(move || content::soundbox_preview(&slug))
        .await
        .map_err(|error| ApiError::server(anyhow::anyhow!("admin request failed: {error}")))?
        .map_err(ApiError::bad_request)?;
    let mut response = wav.into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("audio/wav"),
    );
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("max-age=86400"),
    );
    Ok(response)
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
) -> Result<Json<Vec<events::RecentActivityEventResponse>>, ApiError> {
    blocking(config, move |config| {
        events::recent_button_events(config, token.as_deref())
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
    use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use rusqlite::{params, Connection};
    use serde_json::json;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use crate::config::AdminConfig;
    use crate::db::admin::auth::{
        add_cube_membership, create_session, generate_uuid_v4, hash_password, now, CubeRole,
    };

    fn test_config(root: &TempDir) -> AdminConfig {
        AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: root.path().join("tcube.sqlite3"),
            ui_dist: root.path().join("admin-ui"),
            media_root: root.path().join("audio"),
            content_root: PathBuf::from("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
            usb_connected: true,
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

    #[tokio::test]
    async fn router_serves_admin_index_as_inline_html() {
        let root = TempDir::new().unwrap();
        let ui_dist = root.path().join("admin-ui");
        std::fs::create_dir_all(&ui_dist).unwrap();
        std::fs::write(ui_dist.join("index.html"), "<!doctype html>").unwrap();

        let config = Arc::new(test_config(&root));
        let response = super::router()
            .with_state(config)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

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
    async fn pomodoro_get_defaults_and_owner_save() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));
        let owner_cookie = bootstrap_owner_cookie(app.clone()).await;

        let defaults = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/setup/pomodoro")
                    .header("cookie", &owner_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(defaults.status(), StatusCode::OK);
        let body = to_bytes(defaults.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["enabled"], false);
        assert_eq!(body["focus_minutes"], 10);
        assert_eq!(body["recommendation"]["preset"], "mini");
        assert!(body["validated_at"].is_null());

        let saved = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/pi/v1/setup/pomodoro")
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "enabled": true,
                            "child_age_years": 9,
                            "focus_minutes": 20,
                            "break_minutes": 5,
                            "cycles": 3,
                            "preset": "focus"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        let body = to_bytes(saved.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["enabled"], true);
        assert_eq!(body["child_age_years"], 9);
        assert_eq!(body["recommendation"]["focus_minutes"], 20);
        assert!(body["validated_at"].as_str().is_some());
    }

    #[tokio::test]
    async fn pomodoro_manager_can_view_but_not_save() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));
        let _owner_cookie = bootstrap_owner_cookie(app.clone()).await;
        let manager_cookie = create_manager_cookie(&config.database);

        let view = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/setup/pomodoro")
                    .header("cookie", &manager_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(view.status(), StatusCode::OK);

        let save = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/pi/v1/setup/pomodoro")
                    .header("cookie", &manager_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "enabled": true,
                            "child_age_years": 9,
                            "focus_minutes": 20,
                            "break_minutes": 5,
                            "cycles": 3,
                            "preset": "focus"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(save.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(body["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("owner permission required"));
    }

    #[tokio::test]
    async fn soundbox_mode_catalog_and_toggle_flow() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));
        let owner_cookie = bootstrap_owner_cookie(app.clone()).await;

        let set_mode = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/pi/v1/setup/buttons/4/mode")
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "mode": "soundbox" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(set_mode.status(), StatusCode::OK);

        let review = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/setup/review")
                    .header("cookie", &owner_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(review.status(), StatusCode::OK);
        let body = to_bytes(review.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["button_modes"]["4"], "soundbox");

        let catalog = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/content/buttons/4/soundbox")
                    .header("cookie", &owner_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(catalog.status(), StatusCode::OK);
        let body = to_bytes(catalog.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 6);
        assert!(items.iter().all(|item| item["active"] == true));
        assert_eq!(
            items
                .iter()
                .filter(|item| item["category"] == "bedtime")
                .count(),
            3
        );

        let toggled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/pi/v1/content/buttons/4/soundbox/korobeiniki")
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "active": false }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(toggled.status(), StatusCode::OK);
        let body = to_bytes(toggled.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let korobeiniki = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["slug"] == "korobeiniki")
            .unwrap();
        assert_eq!(korobeiniki["active"], false);
    }

    #[tokio::test]
    async fn soundbox_toggle_keeps_last_active_sound() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));
        let owner_cookie = bootstrap_owner_cookie(app.clone()).await;

        let slugs = [
            "twinkle-twinkle",
            "brahms-lullaby",
            "rock-a-bye-baby",
            "korobeiniki",
            "mountain-king",
        ];
        for slug in slugs {
            let toggled = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/pi/v1/content/buttons/2/soundbox/{slug}"))
                        .header("cookie", &owner_cookie)
                        .header("content-type", "application/json")
                        .body(Body::from(json!({ "active": false }).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(toggled.status(), StatusCode::OK);
        }

        let last = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/pi/v1/content/buttons/2/soundbox/flight-of-the-bumblebee")
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "active": false }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(last.status(), StatusCode::BAD_REQUEST);

        let unknown = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/pi/v1/content/buttons/2/soundbox/not-a-melody")
                    .header("cookie", &owner_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "active": false }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn soundbox_catalog_requires_authentication() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));
        let _owner_cookie = bootstrap_owner_cookie(app.clone()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/content/buttons/4/soundbox")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn soundbox_preview_returns_wav_audio() {
        let root = TempDir::new().unwrap();
        let config = Arc::new(test_config(&root));
        let app = super::router().with_state(Arc::clone(&config));

        let preview = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/content/soundbox/twinkle-twinkle/preview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview.status(), StatusCode::OK);
        assert_eq!(preview.headers().get(CONTENT_TYPE).unwrap(), "audio/wav");
        let body = to_bytes(preview.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(&body[0..4], b"RIFF");

        let unknown = app
            .oneshot(
                Request::builder()
                    .uri("/api/pi/v1/content/soundbox/not-a-melody/preview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
    }

    async fn bootstrap_owner_cookie(app: Router) -> String {
        let response = app
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
        assert_eq!(response.status(), StatusCode::OK);
        response
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    fn create_manager_cookie(database: &PathBuf) -> String {
        let conn = Connection::open(database).unwrap();
        let device_id: String = conn
            .query_row(
                "select device_id from device_setup where id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let account_id = generate_uuid_v4();
        conn.execute(
            "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
             values (?1, 'manager', 'Manager', ?2, ?3)",
            params![
                account_id,
                hash_password("manager-password").unwrap(),
                now()
            ],
        )
        .unwrap();
        add_cube_membership(&conn, &account_id, &device_id, CubeRole::Manager).unwrap();
        let token = create_session(&conn, &account_id).unwrap();
        format!("tcube_session={token}")
    }
}
