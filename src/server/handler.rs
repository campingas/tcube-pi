use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use axum::body::{to_bytes, Body};
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::AdminConfig;
use crate::db::admin::auth::{
    account_by_username, account_password_hash, authenticate_session, create_session,
    ensure_first_account_owner_membership, first_cube_identity, generate_uuid_v4, hash_password,
    local_cubes, local_device_id, normalize_username, now, random_token, require_local_cube_role,
    revoke_all_sessions, session_id_for_token, sha256_hex, timestamp, verify_password, LocalCube,
    RoleRequirement,
};
#[cfg(test)]
use crate::db::admin::auth::{add_cube_membership, session_expires_at, CubeRole};
use crate::db::admin::content::{self as content_storage, ContentEmptyState, NewContentItem};
#[cfg(test)]
use crate::db::admin::schema::migrate_admin_database;
use crate::db::admin::schema::{
    open_admin_database, open_existing_database, table_count, table_exists,
};
use crate::db::admin::setup::{self as setup_storage, SetupReview};

const SESSION_COOKIE_NAME: &str = "tcube_session";
const SESSION_MAX_AGE_SECONDS: i64 = 90 * 24 * 60 * 60;
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 25 * 1024 * 1024;
const SPEECH_PROVIDER_HEALTH_TTL: Duration = Duration::from_secs(20);
const SPEECH_PROVIDER_HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const FACTORY_RESET_CONFIRMATION: &str = "FACTORY RESET";

static SPEECH_PROVIDER_HEALTH_CACHE: OnceLock<Mutex<HashMap<String, CachedSpeechProviderHealth>>> =
    OnceLock::new();

#[derive(Debug, Serialize)]
pub(crate) struct StatusResponse {
    status: &'static str,
    service: &'static str,
    mode: &'static str,
    database_present: bool,
    ui_dist_present: bool,
    media_root: String,
    content_root: String,
    hostname: String,
    usb_address: String,
    contract_note: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthSessionResponse {
    authenticated: bool,
    bootstrap_required: bool,
    account: Option<AccountResponse>,
    cubes: Vec<CubeResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AccountResponse {
    id: String,
    username: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CubeResponse {
    device_id: String,
    label: String,
    role: String,
}

impl From<LocalCube> for CubeResponse {
    fn from(cube: LocalCube) -> Self {
        Self {
            device_id: cube.device_id,
            label: cube.label,
            role: cube.role,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct SetupReviewResponse {
    cube_name: String,
    device_id: Option<String>,
    admin_created: bool,
    wifi_verified: bool,
    dashboard_ip: Option<String>,
    dashboard_address: String,
    button_modes: HashMap<String, String>,
    active_counts: HashMap<String, i64>,
}

impl From<SetupReview> for SetupReviewResponse {
    fn from(review: SetupReview) -> Self {
        Self {
            cube_name: review.cube_name,
            device_id: review.device_id,
            admin_created: review.admin_created,
            wifi_verified: review.wifi_verified,
            dashboard_ip: review.dashboard_ip,
            dashboard_address: review.dashboard_address,
            button_modes: review.button_modes,
            active_counts: review.active_counts,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CubeSaveResponse {
    status: &'static str,
    device_id: String,
    name: String,
    provisioned: bool,
    token: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CompleteSetupResponse {
    status: &'static str,
    led_pattern: &'static str,
    spoken_confirmation: bool,
    dashboard_address: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct FactoryResetResponse {
    status: &'static str,
    bootstrap_required: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActiveContentResponse {
    id: String,
    content_type: String,
    title: String,
    text: String,
    source: String,
    state: &'static str,
    audio_path: Option<String>,
    preview_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct InactiveContentResponse {
    id: String,
    content_type: String,
    title: String,
    text: Option<String>,
    language: Option<String>,
    state: &'static str,
    source: String,
    audio_path: String,
    preview_url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentEmptyStateResponse {
    title: String,
    detail: String,
}

impl From<ContentEmptyState> for ContentEmptyStateResponse {
    fn from(empty_state: ContentEmptyState) -> Self {
        Self {
            title: empty_state.title,
            detail: empty_state.detail,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentListResponse<T> {
    items: Vec<T>,
    empty_state: Option<ContentEmptyStateResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentInventoryResponse {
    items: Vec<ContentInventoryItemResponse>,
    active_count: usize,
    draft_count: usize,
    unused_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentInventoryItemResponse {
    id: String,
    status: String,
    button_id: i64,
    content_type: String,
    language: Option<String>,
    title: String,
    text: Option<String>,
    source: String,
    state: String,
    audio_path: Option<String>,
    preview_url: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CleanupResponse {
    status: &'static str,
    deleted_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct GeneratedSpeechStatusResponse {
    online: bool,
    provider: String,
    checked_at: String,
    cached: bool,
    cache_ttl_seconds: u64,
    next_check_after_seconds: u64,
    message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecentButtonEventResponse {
    occurred_at: String,
    button_id: i64,
    mode: String,
    response_id: String,
    response_text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BootstrapRequest {
    username: String,
    display_name: Option<String>,
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NameRequest {
    cube_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WifiRequest {
    ssid: String,
    dashboard_ip: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ButtonModeRequest {
    mode: String,
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FactoryResetRequest {
    confirmation: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedCleanupRequest {
    button_id: i64,
    language: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedSpeechRequest {
    button_id: i64,
    language: String,
    text: String,
    provider: Option<String>,
    voice: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecoverRequest {
    code: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecoveryCodeResponse {
    code: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvitationCreateRequest {
    device_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvitationAcceptRequest {
    code: String,
    username: String,
    display_name: Option<String>,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct InvitationResponse {
    id: String,
    code: String,
    device_id: String,
    role: &'static str,
    expires_at: String,
}

#[derive(Debug)]
pub(crate) struct MediaInput {
    content_type: String,
    button_id: i64,
    title: String,
    text: String,
    language: String,
    audio_bytes: Vec<u8>,
    original_filename: String,
    mime_type: String,
}

#[derive(Debug)]
pub(crate) struct NormalizedMediaInput {
    content_type: String,
    button_id: i64,
    title: String,
    text: String,
    language: String,
}

#[derive(Debug)]
pub(crate) struct WavInspection {
    duration_seconds: f64,
    peak: f64,
    rms: f64,
}

#[derive(Debug)]
pub(crate) struct GeneratedAudio {
    bytes: Vec<u8>,
    extension: &'static str,
    model: String,
}

#[derive(Clone, Debug)]
struct CachedSpeechProviderHealth {
    online: bool,
    provider: String,
    checked_at: String,
    checked_instant: Instant,
    message: String,
}

pub(crate) fn pi_status(config: &AdminConfig) -> StatusResponse {
    StatusResponse {
        status: "ok",
        service: "tcube-pi-admin",
        mode: "pi_hosted_admin_spike",
        database_present: config.database.exists(),
        ui_dist_present: config.ui_dist.join("index.html").exists(),
        media_root: config.media_root.display().to_string(),
        content_root: config.content_root.display().to_string(),
        hostname: config.hostname.clone(),
        usb_address: config.usb_address.clone(),
        contract_note: "Serves the static admin UI and compatible auth, setup, content, media, and status APIs behind the selected Caddy HTTPS boundary.",
    }
}

pub(crate) fn auth_session(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<(AuthSessionResponse, Option<String>)> {
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok((
            AuthSessionResponse {
                authenticated: false,
                bootstrap_required: true,
                account: None,
                cubes: Vec::new(),
            },
            None,
        ));
    };

    if let Some(session) = authenticate_session(&conn, token)? {
        ensure_first_account_owner_membership(&conn, &session.account.id)?;
        let cubes = local_cubes(&conn, &session.account.id)?
            .into_iter()
            .map(Into::into)
            .collect();
        return Ok((
            AuthSessionResponse {
                authenticated: true,
                bootstrap_required: false,
                account: Some(AccountResponse {
                    id: session.account.id,
                    username: session.account.username,
                    display_name: session.account.display_name,
                }),
                cubes,
            },
            token.map(session_cookie),
        ));
    }

    let account_count = table_count(&conn, "admin_accounts")?;
    Ok((
        AuthSessionResponse {
            authenticated: false,
            bootstrap_required: account_count == 0,
            account: None,
            cubes: Vec::new(),
        },
        None,
    ))
}

pub(crate) fn login_password(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<(AuthSessionResponse, String)> {
    let body: LoginRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let account =
        account_by_username(&conn, &body.username)?.context("invalid username or password")?;
    let password_hash =
        account_password_hash(&conn, &account.id)?.context("invalid username or password")?;
    if !verify_password(&body.password, &password_hash)? {
        anyhow::bail!("invalid username or password");
    }
    let token = create_session(&conn, &account.id)?;
    ensure_first_account_owner_membership(&conn, &account.id)?;
    let cubes = local_cubes(&conn, &account.id)?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account.id,
                username: account.username,
                display_name: account.display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn bootstrap_owner(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<(AuthSessionResponse, String)> {
    let body: BootstrapRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let username = normalize_username(&body.username)?;
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let display_name = body
        .display_name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let display_name = if display_name.is_empty() {
        username.clone()
    } else {
        display_name
    };

    let conn = open_admin_database(config)?;
    let account_count = table_count(&conn, "admin_accounts")?;
    if account_count > 0 {
        anyhow::bail!("local owner already exists");
    }

    let account_id = generate_uuid_v4();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            account_id,
            username,
            display_name,
            hash_password(&body.password)?,
            now()
        ],
    )?;

    let (device_id, cube_name) = first_cube_identity(&conn)?;
    conn.execute(
        "insert into devices (id, label, token_hash, created_at, revoked_at) \
         values (?1, ?2, ?3, ?4, null) \
         on conflict(id) do update set label = excluded.label",
        params![device_id, cube_name, "0".repeat(64), now()],
    )?;
    conn.execute(
        "update device_setup set device_id = ?1, updated_at = ?2 where id = 1",
        params![device_id, now()],
    )?;
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, 'owner', ?3)",
        params![account_id, device_id, now()],
    )?;

    let token = create_session(&conn, &account_id)?;
    let cubes = local_cubes(&conn, &account_id)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account_id,
                username,
                display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn recover_password(config: &AdminConfig, request: &HttpRequest) -> Result<()> {
    let body: RecoverRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let code_hash = sha256_hex(&body.code);
    let row = conn
        .prepare(
            "select id, account_id from recovery_codes \
             where code_hash = ?1 and used_at is null and expires_at > ?2",
        )?
        .query_row(params![code_hash, now()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?
        .context("recovery code is invalid or expired")?;
    let password_hash = hash_password(&body.password)?;

    conn.execute(
        "update admin_accounts set password_hash = ?1 where id = ?2",
        params![password_hash, row.1],
    )?;
    conn.execute(
        "update recovery_codes set used_at = ?1 where id = ?2",
        params![now(), row.0],
    )?;
    revoke_all_sessions(&conn, &row.1)?;
    Ok(())
}

pub(crate) fn create_recovery_code(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<RecoveryCodeResponse> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    let created_at = now();
    conn.execute(
        "update recovery_codes set used_at = ?1 where account_id = ?2 and used_at is null",
        params![created_at, session.account.id],
    )?;
    let code = random_token(24)?;
    let expires_at = timestamp(Utc::now() + chrono::Duration::days(30));
    conn.execute(
        "insert into recovery_codes (id, account_id, code_hash, created_at, expires_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            generate_uuid_v4(),
            session.account.id,
            sha256_hex(&code),
            created_at,
            expires_at
        ],
    )?;
    Ok(RecoveryCodeResponse { code, expires_at })
}

pub(crate) fn create_invitation(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<InvitationResponse> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, request.session_cookie())? else {
        anyhow::bail!("authentication required");
    };
    let body: InvitationCreateRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let device_id = body.device_id.trim();
    if device_id.is_empty() {
        anyhow::bail!("device_id is required");
    }
    let local_device_id = local_device_id(&conn)?;
    if device_id != local_device_id {
        anyhow::bail!("manager invitations can only target the local cube");
    }
    require_local_cube_role(&conn, &session.account.id, RoleRequirement::Owner)?;

    let id = generate_uuid_v4();
    let code = random_token(24)?;
    let expires_at = timestamp(Utc::now() + chrono::Duration::days(7));
    conn.execute(
        "insert into cube_invitations \
         (id, device_id, invited_by, role, code_hash, created_at, expires_at) \
         values (?1, ?2, ?3, 'manager', ?4, ?5, ?6)",
        params![
            id,
            device_id,
            session.account.id,
            sha256_hex(&code),
            now(),
            expires_at
        ],
    )?;
    Ok(InvitationResponse {
        id,
        code,
        device_id: device_id.to_string(),
        role: "manager",
        expires_at,
    })
}

pub(crate) fn accept_invitation(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<(AuthSessionResponse, String)> {
    let body: InvitationAcceptRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let username = normalize_username(&body.username)?;
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let display_name = body
        .display_name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let display_name = if display_name.is_empty() {
        username.clone()
    } else {
        display_name
    };
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let code_hash = sha256_hex(body.code.trim());
    let Some(invitation) = conn
        .prepare(
            "select id, device_id from cube_invitations \
             where code_hash = ?1 and accepted_at is null and revoked_at is null and expires_at > ?2",
        )?
        .query_row(params![code_hash, now()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?
    else {
        anyhow::bail!("invitation is invalid or expired");
    };

    let account_id = generate_uuid_v4();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            account_id,
            username,
            display_name,
            hash_password(&body.password)?,
            now()
        ],
    )
    .context("failed to create manager account")?;
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, 'manager', ?3)",
        params![account_id, invitation.1, now()],
    )?;
    conn.execute(
        "update cube_invitations set accepted_at = ?1, accepted_by = ?2 where id = ?3",
        params![now(), account_id, invitation.0],
    )?;
    let token = create_session(&conn, &account_id)?;
    let cubes = local_cubes(&conn, &account_id)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account_id,
                username,
                display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn set_cube_name(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<CubeSaveResponse> {
    let conn = owner_connection(config, request)?;
    let session = authenticate_session(&conn, request.session_cookie())?
        .context("authentication required")?;
    let body: NameRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let name = body.cube_name.trim();
    if name.is_empty() {
        anyhow::bail!("cube name is required");
    }
    let saved = setup_storage::save_cube_name(&conn, config, &session.account.id, name)?;
    Ok(CubeSaveResponse {
        status: "ok",
        device_id: saved.device_id,
        name: saved.name,
        provisioned: false,
        token: None,
    })
}

pub(crate) fn verify_wifi(config: &AdminConfig, request: &HttpRequest) -> Result<()> {
    let conn = owner_connection(config, request)?;
    let body: WifiRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let ssid = body.ssid.trim();
    let dashboard_ip = body.dashboard_ip.trim();
    if ssid.is_empty() {
        anyhow::bail!("wifi ssid is required");
    }
    if dashboard_ip.is_empty() {
        anyhow::bail!("dashboard ip is required after wifi verification");
    }
    setup_storage::verify_wifi(&conn, config, ssid, dashboard_ip)?;
    Ok(())
}

pub(crate) fn set_button_mode(
    config: &AdminConfig,
    request: &HttpRequest,
    path: &str,
) -> Result<()> {
    let conn = authenticated_connection(config, request)?;
    let button_id = path
        .trim_start_matches("/api/setup/buttons/")
        .trim_end_matches("/mode")
        .trim_matches('/')
        .parse::<i64>()
        .context("button id must be between 1 and 5")?;
    if !(1..=5).contains(&button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let body: ButtonModeRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let mode = body.mode.as_str();
    if !matches!(
        mode,
        "language" | "animals" | "music" | "disabled" | "setup_help"
    ) {
        anyhow::bail!("invalid button mode");
    }
    let selected_language = if mode == "language" {
        Some(
            body.language
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("English")
                .to_string(),
        )
    } else {
        None
    };
    setup_storage::set_button_mode(&conn, button_id, mode, selected_language.as_deref())?;
    Ok(())
}

pub(crate) fn complete_setup(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<CompleteSetupResponse> {
    let conn = owner_connection(config, request)?;
    let review = setup_review_from_conn(config, &conn)?;
    if review.cube_name.trim().is_empty() {
        anyhow::bail!("cube name is required before setup completion");
    }
    if !review.admin_created {
        anyhow::bail!("admin credential is required before setup completion");
    }
    if !review.wifi_verified {
        anyhow::bail!("verified wifi is required before setup completion");
    }
    if review.active_counts.get("language").copied().unwrap_or(0) < 1 {
        anyhow::bail!("at least one language item must be active");
    }
    if review.active_counts.get("animals").copied().unwrap_or(0) < 1 {
        anyhow::bail!("at least one animal item must be active");
    }
    if review.active_counts.get("music").copied().unwrap_or(0) < 1 {
        anyhow::bail!("at least one music item must be active");
    }
    setup_storage::mark_setup_complete(&conn)?;
    Ok(CompleteSetupResponse {
        status: "complete",
        led_pattern: "soft_green_pulse_3s",
        spoken_confirmation: false,
        dashboard_address: setup_review_from_conn(config, &conn)?.dashboard_address,
    })
}

pub(crate) fn factory_reset(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<FactoryResetResponse> {
    let mut conn = owner_connection(config, request)?;
    let body: FactoryResetRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    if body.confirmation.trim() != FACTORY_RESET_CONFIRMATION {
        anyhow::bail!("factory reset confirmation must be FACTORY RESET");
    }

    delete_factory_reset_media(config)?;
    setup_storage::factory_reset_database(&mut conn, config)?;

    Ok(FactoryResetResponse {
        status: "ok",
        bootstrap_required: true,
    })
}

pub(crate) fn save_multipart_media(
    config: &AdminConfig,
    request: &HttpRequest,
    source: &str,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, request)?;
    let input = media_input_from_multipart(request)?;
    let normalized = normalize_media_input(&input, source)?;
    if input.audio_bytes.len() > MAX_AUDIO_BYTES {
        anyhow::bail!("{source} audio must be 25 MB or smaller");
    }
    let extension = if source == "recorded" {
        let wav = inspect_wav(&input.audio_bytes)?;
        validate_wav(&wav, &normalized.content_type)?;
        "wav"
    } else {
        let extension = uploaded_audio_extension(&input.original_filename, &input.mime_type)?;
        if extension == "wav" {
            let wav = inspect_wav(&input.audio_bytes)?;
            validate_wav(&wav, &normalized.content_type)?;
        }
        extension
    };
    let filename = media_filename(
        source,
        &normalized.content_type,
        &normalized.language,
        if normalized.content_type == "language" {
            &normalized.text
        } else {
            &normalized.title
        },
        extension,
    );
    let title = if normalized.content_type == "language" {
        filename.clone()
    } else {
        normalized.title.clone()
    };
    let relative_path = draft_audio_path(&normalized.content_type, &filename);
    let absolute_path = config
        .media_root
        .join("draft")
        .join(&normalized.content_type)
        .join(&filename);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create media directory {}", parent.display()))?;
    }
    fs::write(&absolute_path, &input.audio_bytes)
        .with_context(|| format!("failed to write media file {}", absolute_path.display()))?;

    let item_id = format!("{source}-{}-{}", normalized.content_type, random_token(12)?);
    let order_index =
        content_storage::next_order_index(&conn, &normalized.content_type, normalized.button_id)?;
    let text = if normalized.content_type == "language" {
        normalized.text.clone()
    } else {
        normalized.title.clone()
    };
    content_storage::insert_content_item(
        &conn,
        &NewContentItem {
            id: &item_id,
            content_type: &normalized.content_type,
            button_id: normalized.button_id,
            language: empty_to_null(&normalized.language),
            title: &title,
            text: &text,
            audio_path: &relative_path,
            source,
            order_index,
        },
    )?;
    content_storage::insert_media_artifact_if_present(
        &conn,
        &item_id,
        source,
        &relative_path,
        None,
    )?;
    inactive_response_for_item(&conn, &item_id)
}

pub(crate) fn save_generated_speech(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, request)?;
    let body: GeneratedSpeechRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    let text = body.text.trim();
    let language = body.language.trim();
    if !(1..=5).contains(&body.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    if text.is_empty() {
        anyhow::bail!("generated speech text is required");
    }
    if text.len() > 240 {
        anyhow::bail!("generated speech text must be 240 characters or fewer");
    }
    if language.is_empty() {
        anyhow::bail!("language is required");
    }
    let generated = generate_speech_audio(
        body.provider.as_deref().unwrap_or("auto"),
        language,
        text,
        body.voice.as_deref(),
    )?;
    if generated.bytes.len() > MAX_AUDIO_BYTES {
        anyhow::bail!("generated audio must be 25 MB or smaller");
    }
    if generated.extension == "wav" {
        let wav = inspect_wav(&generated.bytes)?;
        validate_wav(&wav, "language")?;
    }
    let filename = generated_filename(&generated.model, language, text, generated.extension);
    let relative_path = draft_audio_path("language", &filename);
    let absolute_path = config
        .media_root
        .join("draft")
        .join("language")
        .join(&filename);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create media directory {}", parent.display()))?;
    }
    fs::write(&absolute_path, &generated.bytes)
        .with_context(|| format!("failed to write media file {}", absolute_path.display()))?;

    let item_id = format!("generated-language-{}", random_token(12)?);
    let order_index = content_storage::next_order_index(&conn, "language", body.button_id)?;
    content_storage::insert_content_item(
        &conn,
        &NewContentItem {
            id: &item_id,
            content_type: "language",
            button_id: body.button_id,
            language: Some(language),
            title: &filename,
            text,
            audio_path: &relative_path,
            source: "generated",
            order_index,
        },
    )?;
    content_storage::insert_media_artifact_if_present(
        &conn,
        &item_id,
        "generated",
        &relative_path,
        Some(text),
    )?;
    inactive_response_for_item(&conn, &item_id)
}

pub(crate) fn generated_speech_status(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<GeneratedSpeechStatusResponse> {
    let _conn = authenticated_connection(config, request)?;
    let language = request
        .query
        .get("language")
        .map(String::as_str)
        .unwrap_or("English")
        .trim();
    let provider = request
        .query
        .get("provider")
        .map(String::as_str)
        .unwrap_or("auto")
        .trim();
    let resolved_provider = resolve_speech_provider(provider, language);
    let base_url = speech_provider_base_url(resolved_provider)?;
    let cache_key = format!("{resolved_provider}:{base_url}");
    let cached = cached_speech_provider_health(cache_key, resolved_provider.to_string(), || {
        probe_speech_provider(&base_url)
    })?;
    Ok(speech_provider_status_response(cached))
}

pub(crate) fn list_active_content(
    config: &AdminConfig,
    request: &HttpRequest,
    path: &str,
) -> Result<ContentListResponse<ActiveContentResponse>> {
    let conn = authenticated_connection(config, request)?;
    let (button_id, content_type) = parse_content_button_path(path, "active")?;
    let language = request
        .query
        .get("language")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let items = content_storage::active_content_rows(&conn, button_id, content_type, language)?;
    let empty_state = if items.is_empty() {
        content_storage::content_empty_state(&conn, button_id, content_type, language, "active")?
            .map(Into::into)
    } else {
        None
    };
    let items = items
        .into_iter()
        .map(|item| {
            let title = item.title.unwrap_or_else(|| item.id.clone());
            let text = item.text.unwrap_or_else(|| title.clone());
            ActiveContentResponse {
                id: item.id,
                content_type: item.content_type,
                title,
                text,
                source: item.source,
                state: "active",
                preview_url: item.audio_path.as_deref().map(content_preview_url),
                audio_path: item.audio_path,
            }
        })
        .collect();
    Ok(ContentListResponse { items, empty_state })
}

pub(crate) fn list_inactive_content(
    config: &AdminConfig,
    request: &HttpRequest,
    path: &str,
) -> Result<ContentListResponse<InactiveContentResponse>> {
    let conn = authenticated_connection(config, request)?;
    let (button_id, content_type) = parse_content_button_path(path, "inactive")?;
    let language = if content_type == "language" {
        request
            .query
            .get("language")
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    let rows = content_storage::inactive_content_rows(&conn, button_id, content_type, language)?;
    let empty_state = if rows.is_empty() {
        content_storage::content_empty_state(&conn, button_id, content_type, language, "archived")?
            .map(Into::into)
    } else {
        None
    };
    let items = rows
        .into_iter()
        .map(|item| {
            let title = item.title.unwrap_or_else(|| item.id.clone());
            InactiveContentResponse {
                id: item.id,
                content_type: item.content_type.clone(),
                title: title.clone(),
                text: if item.content_type == "music" {
                    None
                } else {
                    Some(item.text.unwrap_or_else(|| title.clone()))
                },
                language: item.language,
                state: "archived",
                source: item.source,
                preview_url: content_preview_url(&item.audio_path.clone().unwrap_or_default()),
                audio_path: item.audio_path.unwrap_or_default(),
            }
        })
        .collect();
    Ok(ContentListResponse { items, empty_state })
}

pub(crate) fn content_inventory(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<ContentInventoryResponse> {
    let conn = authenticated_connection(config, request)?;
    let rows = content_storage::content_inventory_rows(&conn)?;
    let mut active_count = 0;
    let mut draft_count = 0;
    let mut unused_count = 0;
    let items = rows
        .into_iter()
        .map(|item| {
            let (status, reason) = content_storage::inventory_status(&conn, &item)?;
            match status {
                "active" => active_count += 1,
                "draft" => draft_count += 1,
                "unused" => unused_count += 1,
                _ => {}
            }
            Ok(ContentInventoryItemResponse {
                id: item.id,
                status: status.to_string(),
                button_id: item.button_id,
                content_type: item.content_type,
                language: item.language,
                title: item.title.unwrap_or_else(|| "Untitled audio".to_string()),
                text: item.text,
                source: item.source,
                state: item.state,
                preview_url: item.audio_path.as_deref().map(content_preview_url),
                audio_path: item.audio_path,
                reason,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ContentInventoryResponse {
        items,
        active_count,
        draft_count,
        unused_count,
    })
}

pub(crate) fn trash_unused_content(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<CleanupResponse> {
    let conn = owner_connection(config, request)?;
    let unused = content_storage::unused_content_items(&conn)?;
    let audio_paths = unused
        .iter()
        .filter_map(|item| item.audio_path.clone())
        .collect::<Vec<_>>();
    for audio_path in &audio_paths {
        delete_content_audio_file(config, audio_path)?;
    }
    let trashed_at = now();
    let purge_after_at = purge_after();
    content_storage::trash_content_items(&conn, &unused, &trashed_at, &purge_after_at)?;
    Ok(CleanupResponse {
        status: "ok",
        deleted_count: unused.len(),
    })
}

pub(crate) fn activate_content_item(
    config: &AdminConfig,
    request: &HttpRequest,
    path: &str,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, request)?;
    let item_id = path
        .trim_start_matches("/api/content/items/")
        .trim_end_matches("/activate")
        .trim_matches('/');
    let item =
        content_storage::content_item_by_id(&conn, item_id)?.context("content item not found")?;
    if !matches!(item.source.as_str(), "recorded" | "uploaded" | "generated") {
        anyhow::bail!(
            "only inactive recorded, uploaded, or generated content can be activated here"
        );
    }
    if item.state != "archived" {
        anyhow::bail!("content is not inactive");
    }
    let audio_path = item.audio_path.clone().unwrap_or_default();
    let next_audio_path = activate_audio_file(config, &audio_path)?;
    content_storage::activate_content_item(&conn, &item.id, &next_audio_path)?;
    Ok(InactiveContentResponse {
        id: item.id,
        content_type: item.content_type.clone(),
        title: item.title.unwrap_or_else(|| item_id.to_string()),
        text: if item.content_type == "music" {
            None
        } else {
            item.text.or_else(|| Some(item_id.to_string()))
        },
        language: item.language,
        state: "active",
        source: item.source,
        preview_url: content_preview_url(&next_audio_path),
        audio_path: next_audio_path,
    })
}

pub(crate) fn trash_content_item(
    config: &AdminConfig,
    request: &HttpRequest,
    path: &str,
) -> Result<()> {
    let conn = authenticated_connection(config, request)?;
    let item_id = path
        .trim_start_matches("/api/content/items/")
        .trim_matches('/');
    let item =
        content_storage::content_item_by_id(&conn, item_id)?.context("content item not found")?;
    delete_draft_audio_file(config, item.audio_path.as_deref())?;
    let trashed_at = now();
    let changes = content_storage::trash_content_item(&conn, item_id, &trashed_at, &purge_after())?;
    if changes == 0 {
        anyhow::bail!("content item not found: {item_id}");
    }
    Ok(())
}

pub(crate) fn trash_unused_generated_speech(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<CleanupResponse> {
    let conn = authenticated_connection(config, request)?;
    let body: GeneratedCleanupRequest =
        serde_json::from_slice(&request.body).context("invalid request body")?;
    if !(1..=5).contains(&body.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let language = body.language.trim();
    if language.is_empty() {
        anyhow::bail!("language is required");
    }
    let draft_paths =
        content_storage::draft_audio_paths_for_cleanup(&conn, body.button_id, language)?;
    delete_draft_audio_files(config, &draft_paths)?;
    let trashed_at = now();
    let deleted_count = content_storage::trash_generated_language_drafts(
        &conn,
        body.button_id,
        language,
        &trashed_at,
        &purge_after(),
    )?;
    Ok(CleanupResponse {
        status: "ok",
        deleted_count,
    })
}

pub(crate) fn logout(config: &AdminConfig, token: Option<&str>) -> Result<()> {
    let Some(token) = token else {
        return Ok(());
    };
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok(());
    };
    if let Some(session_id) = session_id_for_token(&conn, token)? {
        conn.execute(
            "update admin_sessions set revoked_at = ?1 where id = ?2",
            params![now(), session_id],
        )?;
    }
    Ok(())
}

pub(crate) fn recent_button_events(
    config: &AdminConfig,
    request: &HttpRequest,
) -> Result<Vec<RecentButtonEventResponse>> {
    let conn = authenticated_connection(config, request)?;
    if !table_exists(&conn, "button_events")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "select occurred_at, button_id, mode, response_id, response_text \
         from button_events order by id desc limit 20",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecentButtonEventResponse {
            occurred_at: row.get(0)?,
            button_id: row.get(1)?,
            mode: row.get(2)?,
            response_id: row.get(3)?,
            response_text: row.get(4)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn authenticated_connection(config: &AdminConfig, request: &HttpRequest) -> Result<Connection> {
    role_authorized_connection(config, request, RoleRequirement::Member)
}

fn owner_connection(config: &AdminConfig, request: &HttpRequest) -> Result<Connection> {
    role_authorized_connection(config, request, RoleRequirement::Owner)
}

fn role_authorized_connection(
    config: &AdminConfig,
    request: &HttpRequest,
    requirement: RoleRequirement,
) -> Result<Connection> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, request.session_cookie())? else {
        anyhow::bail!("authentication required");
    };
    require_local_cube_role(&conn, &session.account.id, requirement)?;
    Ok(conn)
}

pub(crate) fn setup_review(config: &AdminConfig) -> Result<SetupReviewResponse> {
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok(setup_storage::default_setup_review(config).into());
    };
    setup_review_from_conn(config, &conn)
}

pub(crate) fn setup_review_from_conn(
    config: &AdminConfig,
    conn: &Connection,
) -> Result<SetupReviewResponse> {
    Ok(setup_storage::setup_review_from_conn(config, conn)?.into())
}

pub(crate) fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={SESSION_MAX_AGE_SECONDS}"
    )
}

pub(crate) fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
}

fn inactive_response_for_item(conn: &Connection, item_id: &str) -> Result<InactiveContentResponse> {
    let item =
        content_storage::content_item_by_id(conn, item_id)?.context("content item not found")?;
    let title = item.title.unwrap_or_else(|| item.id.clone());
    let audio_path = item.audio_path.unwrap_or_default();
    Ok(InactiveContentResponse {
        id: item.id,
        content_type: item.content_type.clone(),
        title: title.clone(),
        text: if item.content_type == "music" {
            None
        } else {
            Some(item.text.unwrap_or_else(|| title.clone()))
        },
        language: item.language,
        state: if item.state == "active" {
            "active"
        } else {
            "archived"
        },
        source: item.source,
        preview_url: content_preview_url(&audio_path),
        audio_path,
    })
}

fn media_input_from_multipart(request: &HttpRequest) -> Result<MediaInput> {
    let content_type = request
        .headers
        .get("content-type")
        .context("multipart content type is required")?;
    let boundary = content_type
        .split(';')
        .find_map(|part| part.trim().strip_prefix("boundary="))
        .map(|value| value.trim_matches('"'))
        .context("multipart boundary is required")?;
    let parts = parse_multipart(&request.body, boundary);
    let field = |name: &str| -> String {
        parts
            .iter()
            .find(|part| part.name == name)
            .map(|part| String::from_utf8_lossy(&part.bytes).to_string())
            .unwrap_or_default()
    };
    let audio = parts
        .iter()
        .find(|part| part.name == "audio_file")
        .context("audio file is required")?;
    Ok(MediaInput {
        content_type: field("content_type"),
        button_id: field("button_id")
            .trim()
            .parse::<i64>()
            .context("button id must be between 1 and 5")?,
        title: field("title"),
        text: field("text"),
        language: field("language"),
        audio_bytes: audio.bytes.clone(),
        original_filename: audio
            .filename
            .clone()
            .unwrap_or_else(|| "upload".to_string()),
        mime_type: audio.content_type.clone().unwrap_or_default(),
    })
}

#[derive(Debug)]
struct MultipartPart {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

fn parse_multipart(body: &[u8], boundary: &str) -> Vec<MultipartPart> {
    let marker = format!("--{boundary}").into_bytes();
    let mut parts = Vec::new();
    for section in split_bytes(body, &marker).into_iter().skip(1) {
        if section.starts_with(b"--") {
            break;
        }
        let section = section.strip_prefix(b"\r\n").unwrap_or(section);
        let Some(header_end) = find_subslice(section, b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&section[..header_end]);
        let mut value = &section[header_end + 4..];
        if let Some(stripped) = value.strip_suffix(b"\r\n") {
            value = stripped;
        }
        let mut name = None;
        let mut filename = None;
        let mut content_type = None;
        for header in headers.split("\r\n") {
            if let Some(disposition) = header.strip_prefix("Content-Disposition:") {
                for item in disposition.split(';').map(str::trim) {
                    if let Some(value) = item.strip_prefix("name=") {
                        name = Some(value.trim_matches('"').to_string());
                    }
                    if let Some(value) = item.strip_prefix("filename=") {
                        filename = Some(value.trim_matches('"').to_string());
                    }
                }
            }
            if let Some(value) = header.strip_prefix("Content-Type:") {
                content_type = Some(value.trim().to_string());
            }
        }
        if let Some(name) = name {
            parts.push(MultipartPart {
                name,
                filename,
                content_type,
                bytes: value.to_vec(),
            });
        }
    }
    parts
}

fn split_bytes<'a>(body: &'a [u8], marker: &[u8]) -> Vec<&'a [u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(offset) = find_subslice(&body[start..], marker) {
        parts.push(&body[start..start + offset]);
        start += offset + marker.len();
    }
    parts.push(&body[start..]);
    parts
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn normalize_media_input(input: &MediaInput, source: &str) -> Result<NormalizedMediaInput> {
    if !matches!(
        input.content_type.as_str(),
        "language" | "animals" | "music"
    ) {
        anyhow::bail!("unsupported {source} content type");
    }
    if !(1..=5).contains(&input.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let title = input.title.trim();
    let text = input.text.trim();
    let language = input.language.trim();
    if input.content_type == "language" && language.is_empty() {
        anyhow::bail!("language {source}s require language");
    }
    if input.content_type == "language" && text.is_empty() {
        anyhow::bail!("language {source}s require spoken text");
    }
    if input.content_type != "language" && title.is_empty() {
        anyhow::bail!("{source} title is required");
    }
    Ok(NormalizedMediaInput {
        content_type: input.content_type.clone(),
        button_id: input.button_id,
        title: title.to_string(),
        text: text.to_string(),
        language: language.to_string(),
    })
}

fn parse_content_button_path<'a>(path: &'a str, suffix: &str) -> Result<(i64, &'a str)> {
    let parts = path
        .trim_start_matches("/api/content/buttons/")
        .split('/')
        .collect::<Vec<_>>();
    if parts.len() != 3 || parts[2] != suffix {
        anyhow::bail!("invalid content path");
    }
    let button_id = parts[0]
        .parse::<i64>()
        .context("button id must be between 1 and 5")?;
    if !(1..=5).contains(&button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let content_type = parts[1];
    if !matches!(content_type, "language" | "animals" | "music") {
        anyhow::bail!("unsupported content type");
    }
    Ok((button_id, content_type))
}

fn content_preview_url(audio_path: &str) -> String {
    audio_path
        .strip_prefix("data/audio/")
        .map(|path| format!("/api/media/{path}"))
        .or_else(|| {
            audio_path
                .strip_prefix("data/media/")
                .map(|path| format!("/api/media/{path}"))
        })
        .unwrap_or_else(|| format!("/{audio_path}"))
}

fn draft_audio_path(content_type: &str, filename: &str) -> String {
    format!("data/audio/draft/{content_type}/{filename}")
}

fn active_audio_path_from_draft(audio_path: &str) -> Option<String> {
    audio_path
        .strip_prefix("data/audio/draft/")
        .map(|path| format!("data/audio/active/{path}"))
}

fn activate_audio_file(config: &AdminConfig, audio_path: &str) -> Result<String> {
    let Some(active_path) = active_audio_path_from_draft(audio_path) else {
        return Ok(audio_path.to_string());
    };
    let draft_relative = audio_path
        .strip_prefix("data/audio/")
        .context("draft audio path must be under data/audio")?;
    let active_relative = active_path
        .strip_prefix("data/audio/")
        .context("active audio path must be under data/audio")?;
    let draft_absolute = config.media_root.join(draft_relative);
    let active_absolute = config.media_root.join(active_relative);
    if let Some(parent) = active_absolute.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create active audio directory {}",
                parent.display()
            )
        })?;
    }
    fs::rename(&draft_absolute, &active_absolute).with_context(|| {
        format!(
            "failed to move draft audio {} to active audio {}",
            draft_absolute.display(),
            active_absolute.display()
        )
    })?;
    Ok(active_path)
}

fn draft_audio_absolute_path(config: &AdminConfig, audio_path: &str) -> Option<std::path::PathBuf> {
    let draft_relative = audio_path.strip_prefix("data/audio/draft/")?;
    Some(config.media_root.join("draft").join(draft_relative))
}

fn delete_draft_audio_file(config: &AdminConfig, audio_path: Option<&str>) -> Result<()> {
    let Some(audio_path) = audio_path else {
        return Ok(());
    };
    let Some(path) = draft_audio_absolute_path(config, audio_path) else {
        return Ok(());
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to delete draft audio file {}", path.display())),
    }
}

fn delete_draft_audio_files(config: &AdminConfig, audio_paths: &[String]) -> Result<()> {
    for audio_path in audio_paths {
        delete_draft_audio_file(config, Some(audio_path))?;
    }
    Ok(())
}

fn content_audio_absolute_path(
    config: &AdminConfig,
    audio_path: &str,
) -> Option<std::path::PathBuf> {
    let relative = audio_path.strip_prefix("data/audio/")?;
    Some(config.media_root.join(relative))
}

fn delete_content_audio_file(config: &AdminConfig, audio_path: &str) -> Result<()> {
    let Some(path) = content_audio_absolute_path(config, audio_path) else {
        return Ok(());
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to delete audio file {}", path.display()))
        }
    }
}

fn delete_factory_reset_media(config: &AdminConfig) -> Result<()> {
    for directory in ["draft", "active"] {
        let path = config.media_root.join(directory);
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to delete factory reset media directory {}",
                        path.display()
                    )
                });
            }
        }
    }
    Ok(())
}

fn purge_after() -> String {
    timestamp(Utc::now() + chrono::Duration::days(15))
}

fn uploaded_audio_extension(filename: &str, mime_type: &str) -> Result<&'static str> {
    let filename = filename.to_ascii_lowercase();
    let mime_type = mime_type.to_ascii_lowercase();
    if filename.ends_with(".wav") || mime_type.contains("wav") {
        return Ok("wav");
    }
    if filename.ends_with(".mp3") || mime_type.contains("mpeg") || mime_type.contains("mp3") {
        return Ok("mp3");
    }
    anyhow::bail!("uploaded audio must be an MP3 or WAV file");
}

fn validate_wav(wav: &WavInspection, content_type: &str) -> Result<()> {
    let max_duration = if content_type == "music" { 180.0 } else { 15.0 };
    if wav.duration_seconds > max_duration {
        if content_type == "music" {
            anyhow::bail!("music audio must be 3 minutes or shorter");
        }
        anyhow::bail!("language and animal audio must be 15 seconds or shorter");
    }
    if wav.peak < 0.02 || wav.rms < 0.005 {
        anyhow::bail!("audio is too quiet");
    }
    Ok(())
}

fn inspect_wav(bytes: &[u8]) -> Result<WavInspection> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        anyhow::bail!("recorded audio must be a WAV file");
    }
    let mut offset = 12_usize;
    let mut audio_format = 0_u16;
    let mut channels = 0_u16;
    let mut sample_rate = 0_u32;
    let mut bits_per_sample = 0_u16;
    let mut data_offset = None;
    let mut data_size = 0_usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let chunk_data_offset = offset + 8;
        if chunk_data_offset + chunk_size > bytes.len() {
            anyhow::bail!("recorded WAV file is malformed");
        }
        if chunk_id == b"fmt " {
            audio_format = u16::from_le_bytes(
                bytes[chunk_data_offset..chunk_data_offset + 2]
                    .try_into()
                    .unwrap(),
            );
            channels = u16::from_le_bytes(
                bytes[chunk_data_offset + 2..chunk_data_offset + 4]
                    .try_into()
                    .unwrap(),
            );
            sample_rate = u32::from_le_bytes(
                bytes[chunk_data_offset + 4..chunk_data_offset + 8]
                    .try_into()
                    .unwrap(),
            );
            bits_per_sample = u16::from_le_bytes(
                bytes[chunk_data_offset + 14..chunk_data_offset + 16]
                    .try_into()
                    .unwrap(),
            );
        } else if chunk_id == b"data" {
            data_offset = Some(chunk_data_offset);
            data_size = chunk_size;
            break;
        }
        offset = chunk_data_offset + chunk_size + (chunk_size % 2);
    }
    if audio_format != 1 || bits_per_sample != 16 || channels < 1 || sample_rate < 8000 {
        anyhow::bail!("recorded WAV file must be 16-bit PCM audio");
    }
    let data_offset = data_offset.context("recorded WAV file has no audio data")?;
    if data_size < 2 {
        anyhow::bail!("recorded WAV file has no audio data");
    }
    let mut peak = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut samples = 0_usize;
    for sample_offset in (data_offset..data_offset + data_size - 1).step_by(2) {
        let sample = i16::from_le_bytes(bytes[sample_offset..sample_offset + 2].try_into().unwrap())
            as f64
            / 32768.0;
        let abs = sample.abs();
        peak = peak.max(abs);
        sum_squares += sample * sample;
        samples += 1;
    }
    Ok(WavInspection {
        duration_seconds: samples as f64 / channels as f64 / sample_rate as f64,
        peak,
        rms: (sum_squares / samples as f64).sqrt(),
    })
}

fn media_filename(
    source: &str,
    content_type: &str,
    language: &str,
    label: &str,
    extension: &str,
) -> String {
    if content_type == "language" {
        format!(
            "{source}-{}-{}-{}.{}",
            slug_part(language),
            slug_part(label),
            recording_timestamp(),
            if source == "recorded" {
                "wav"
            } else {
                extension
            }
        )
    } else {
        format!(
            "{}-{}-{}.{}",
            slug_part(source),
            slug_part(label),
            recording_timestamp(),
            if source == "recorded" {
                "wav"
            } else {
                extension
            }
        )
    }
}

fn generated_filename(model: &str, language: &str, text: &str, extension: &str) -> String {
    let text_slug = slug_part(text);
    let truncated = text_slug.chars().take(72).collect::<String>();
    format!(
        "generated-{}-{}-{}-{}.{}",
        slug_part(model),
        slug_part(language),
        truncated,
        recording_timestamp(),
        extension
    )
}

fn slug_part(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "unknown".to_string()
    } else {
        slug
    }
}

fn recording_timestamp() -> String {
    Utc::now().format("%Y%m%d%H%M%S%3f").to_string()
}

fn empty_to_null(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn generate_speech_audio(
    provider: &str,
    language: &str,
    text: &str,
    voice: Option<&str>,
) -> Result<GeneratedAudio> {
    let provider = resolve_speech_provider(provider, language);
    match provider {
        "voxtral" => {
            let base = speech_provider_base_url(provider)?;
            let model = std::env::var("VOXTRAL_MODEL")
                .unwrap_or_else(|_| "mistralai/Voxtral-4B-TTS-2603".to_string());
            let voice = voice
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| std::env::var("VOXTRAL_VOICE").ok())
                .unwrap_or_else(|| "neutral_male".to_string());
            let body = json!({
                "input": text,
                "model": model,
                "response_format": "wav",
                "voice": voice
            })
            .to_string();
            let bytes = post_speech_json(
                &format!("{}/audio/speech", base.trim_end_matches('/')),
                &body,
                vec![(
                    "Authorization".to_string(),
                    format!(
                        "Bearer {}",
                        std::env::var("VOXTRAL_API_KEY").unwrap_or_else(|_| "EMPTY".to_string())
                    ),
                )],
            )?;
            Ok(GeneratedAudio {
                bytes,
                extension: "wav",
                model: model_name_for_file(&model),
            })
        }
        "vietnamese-vits" => {
            let base = speech_provider_base_url(provider)?;
            let body = json!({ "input": text, "response_format": "wav" }).to_string();
            let bytes = post_speech_json(
                &format!("{}/v1/audio/speech", base.trim_end_matches('/')),
                &body,
                Vec::new(),
            )?;
            Ok(GeneratedAudio {
                bytes,
                extension: "wav",
                model: "vietnamese-vits".to_string(),
            })
        }
        "mistral" => {
            anyhow::bail!("hosted Mistral generation is not supported by the Pi Rust spike yet")
        }
        _ => anyhow::bail!("unsupported speech provider"),
    }
}

fn resolve_speech_provider<'a>(provider: &'a str, language: &str) -> &'a str {
    if provider == "auto" {
        if language.eq_ignore_ascii_case("vietnamese") {
            "vietnamese-vits"
        } else {
            "voxtral"
        }
    } else {
        provider
    }
}

fn speech_provider_base_url(provider: &str) -> Result<String> {
    match provider {
        "voxtral" => Ok(std::env::var("VOXTRAL_API_BASE")
            .unwrap_or_else(|_| "https://127.0.0.1:8001/v1".to_string())),
        "vietnamese-vits" => Ok(std::env::var("VIETNAMESE_VITS_API_BASE")
            .unwrap_or_else(|_| "https://127.0.0.1:7872".to_string())),
        "mistral" => {
            anyhow::bail!("hosted Mistral generation is not supported by the Pi Rust spike yet")
        }
        _ => anyhow::bail!("unsupported speech provider"),
    }
}

fn cached_speech_provider_health(
    cache_key: String,
    provider: String,
    probe: impl FnOnce() -> Result<()>,
) -> Result<(CachedSpeechProviderHealth, bool)> {
    let cache = SPEECH_PROVIDER_HEALTH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache
        .lock()
        .expect("speech provider health cache poisoned")
        .get(&cache_key)
        .filter(|cached| cached.checked_instant.elapsed() < SPEECH_PROVIDER_HEALTH_TTL)
        .cloned()
    {
        return Ok((cached, true));
    }

    let checked_at = Utc::now().to_rfc3339();
    let (online, message) = match probe() {
        Ok(()) => (
            true,
            "TTS provider is online and ready for generated speech.".to_string(),
        ),
        Err(error) => (
            false,
            format!("TTS provider is offline or unreachable: {error}"),
        ),
    };
    let health = CachedSpeechProviderHealth {
        online,
        provider,
        checked_at,
        checked_instant: Instant::now(),
        message,
    };
    cache
        .lock()
        .expect("speech provider health cache poisoned")
        .insert(cache_key, health.clone());
    Ok((health, false))
}

fn speech_provider_status_response(
    health: (CachedSpeechProviderHealth, bool),
) -> GeneratedSpeechStatusResponse {
    let (health, cached) = health;
    GeneratedSpeechStatusResponse {
        online: health.online,
        provider: health.provider,
        checked_at: health.checked_at,
        cached,
        cache_ttl_seconds: SPEECH_PROVIDER_HEALTH_TTL.as_secs(),
        next_check_after_seconds: SPEECH_PROVIDER_HEALTH_TTL.as_secs(),
        message: health.message,
    }
}

fn probe_speech_provider(base_url: &str) -> Result<()> {
    let url = validate_speech_api_url(base_url)?;
    let client = speech_http_client_with_timeout(SPEECH_PROVIDER_HEALTH_TIMEOUT)?;
    client
        .get(url)
        .send()
        .with_context(|| format!("failed to connect to speech provider {base_url}"))?;
    Ok(())
}

fn post_speech_json(
    url: &str,
    body: &str,
    extra_headers: Vec<(String, String)>,
) -> Result<Vec<u8>> {
    let url_text = url.to_owned();
    let url = validate_speech_api_url(&url_text)?;
    let client = speech_http_client()?;
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_owned());
    for (name, value) in extra_headers {
        request = request.header(name, value);
    }

    let response = request
        .send()
        .with_context(|| format!("failed to connect to speech provider {url_text}"))?;
    let status = response.status();
    let response_body = response
        .bytes()
        .context("speech provider returned unreadable response body")?;
    if !status.is_success() {
        anyhow::bail!(
            "speech generation failed: {}",
            String::from_utf8_lossy(&response_body)
                .chars()
                .take(500)
                .collect::<String>()
        );
    }
    if response_body.is_empty() {
        anyhow::bail!("speech generation failed: empty audio response");
    }
    Ok(response_body.to_vec())
}

fn validate_speech_api_url(url: &str) -> Result<reqwest::Url> {
    let parsed_url =
        reqwest::Url::parse(url).with_context(|| format!("invalid speech provider URL: {url}"))?;
    if parsed_url.scheme() != "https" {
        anyhow::bail!("speech provider URL must use https: {url}");
    }
    Ok(parsed_url)
}

fn speech_http_client() -> Result<reqwest::blocking::Client> {
    speech_http_client_with_ca_cert_path(
        std::env::var_os("TCUBE_SPEECH_API_CA_CERT")
            .as_deref()
            .map(Path::new),
    )
}

fn speech_http_client_with_timeout(timeout: Duration) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout);
    if let Some(path) = std::env::var_os("TCUBE_SPEECH_API_CA_CERT")
        .as_deref()
        .map(Path::new)
    {
        let pem = fs::read(path).with_context(|| {
            format!(
                "failed to read speech API CA certificate {}",
                path.display()
            )
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).with_context(|| {
            format!(
                "failed to parse speech API CA certificate {}",
                path.display()
            )
        })?;
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .context("failed to build speech API HTTP client")
}

fn speech_http_client_with_ca_cert_path(
    ca_cert_path: Option<&Path>,
) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(30));
    if let Some(path) = ca_cert_path {
        let pem = fs::read(path).with_context(|| {
            format!(
                "failed to read speech API CA certificate {}",
                path.display()
            )
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).with_context(|| {
            format!(
                "failed to parse speech API CA certificate {}",
                path.display()
            )
        })?;
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .context("failed to build speech API HTTP client")
}

fn model_name_for_file(model: &str) -> String {
    model
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(model)
        .to_string()
}

pub(crate) fn json_response<T: Serialize>(status: u16, body: T) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        headers: Vec::new(),
        body: serde_json::to_vec(&body).expect("serializing JSON response should not fail"),
    }
}

pub(crate) fn error_response(status: u16, detail: impl Into<String>) -> HttpResponse {
    json_response(status, json!({ "detail": detail.into() }))
}

#[derive(Debug)]
pub(crate) struct HttpRequest {
    #[cfg(test)]
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: HashMap<String, String>,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

impl HttpRequest {
    pub(crate) async fn from_request(request: Request<Body>) -> Result<Self> {
        let (parts, body) = request.into_parts();
        let body = to_bytes(body, MAX_REQUEST_BODY_BYTES)
            .await
            .context("failed to read HTTP request body")?;
        let headers = parts
            .headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_ascii_lowercase(), value.trim().to_string()))
            })
            .collect();
        Ok(Self {
            #[cfg(test)]
            method: parts.method.as_str().to_string(),
            path: parts.uri.path().to_string(),
            query: parse_query(parts.uri.query()),
            headers,
            body: body.to_vec(),
        })
    }

    pub(crate) fn session_cookie(&self) -> Option<&str> {
        self.headers
            .get("cookie")
            .and_then(|header| cookie_value(header, SESSION_COOKIE_NAME))
    }
}

fn parse_query(query: Option<&str>) -> HashMap<String, String> {
    query
        .unwrap_or("")
        .split('&')
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((percent_decode(key), percent_decode(value)))
        })
        .collect()
}

fn percent_decode(value: &str) -> String {
    let mut bytes = Vec::with_capacity(value.len());
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        match raw[index] {
            b'+' => {
                bytes.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < raw.len() => {
                if let Ok(hex) = std::str::from_utf8(&raw[index + 1..index + 3]) {
                    if let Ok(value) = u8::from_str_radix(hex, 16) {
                        bytes.push(value);
                        index += 3;
                        continue;
                    }
                }
                bytes.push(raw[index]);
                index += 1;
            }
            byte => {
                bytes.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

#[derive(Debug)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) content_type: &'static str,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl HttpResponse {}

impl IntoResponse for HttpResponse {
    fn into_response(self) -> Response {
        let body_len = self.body.len().to_string();
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let headers = response.headers_mut();
        insert_header(
            headers,
            HeaderName::from_static("content-type"),
            self.content_type,
        );
        insert_header(
            headers,
            HeaderName::from_static("content-length"),
            &body_len,
        );
        for (name, value) in self.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(name, value);
            }
        }
        response
    }
}

fn insert_header(headers: &mut axum::http::HeaderMap, name: HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::try_from(value) {
        headers.insert(name, value);
    }
}

fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if key == name {
            Some(value)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::server::routes::route_request;
    use rusqlite::params;
    use tempfile::TempDir;

    #[test]
    fn serves_bundled_content_audio_from_content_root() {
        let database = test_database();
        let config = test_config(database.path());
        let audio_path = config.content_root.join("audio/animals/cow-moo.wav");
        fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
        fs::write(&audio_path, b"RIFFtest").unwrap();

        let response = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/content/audio/animals/cow-moo.wav".to_string(),
                query: HashMap::new(),
                headers: HashMap::new(),
                body: Vec::new(),
            },
            &config,
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "audio/wav");
        assert_eq!(response.body, b"RIFFtest");
    }

    #[test]
    fn rejects_bundled_content_path_traversal() {
        let database = test_database();
        let config = test_config(database.path());

        let response = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/content/../data/tcube.sqlite3".to_string(),
                query: HashMap::new(),
                headers: HashMap::new(),
                body: Vec::new(),
            },
            &config,
        );

        assert_eq!(response.status, 400);
    }

    #[test]
    fn returns_default_setup_review_without_database() {
        let config = AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: PathBuf::from("/tmp/does-not-exist-tcube.sqlite3"),
            ui_dist: PathBuf::from("admin-ui"),
            media_root: PathBuf::from("data/media"),
            content_root: PathBuf::from("content"),
            hostname: "tcube-a7f3.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
        };

        let review = setup_review(&config).unwrap();

        assert_eq!(review.cube_name, "T-Cube");
        assert_eq!(review.dashboard_address, "https://tcube-a7f3.local/");
        assert_eq!(review.button_modes["1"], "language:English");
    }

    #[test]
    fn bootstrap_owner_creates_clean_database_session_and_first_cube_membership() {
        let directory = TempDir::new().unwrap();
        let database = directory.path().join("fresh/data/tcube.sqlite3");
        let config = test_config(&database);

        let initial_session = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::new(),
                body: Vec::new(),
            },
            &config,
        );
        let initial_body: serde_json::Value =
            serde_json::from_slice(&initial_session.body).unwrap();
        assert_eq!(initial_session.status, 200);
        assert_eq!(initial_body["authenticated"], false);
        assert_eq!(initial_body["bootstrap_required"], true);

        let bootstrap = route_request(
            &json_request(
                "POST",
                "/api/auth/bootstrap",
                json!({
                    "username": "Parent",
                    "display_name": "Parent Admin",
                    "password": "owner-password"
                }),
                None,
            ),
            &config,
        );

        assert_eq!(bootstrap.status, 200);
        assert!(database.exists());
        let bootstrap_body: serde_json::Value = serde_json::from_slice(&bootstrap.body).unwrap();
        assert_eq!(bootstrap_body["authenticated"], true);
        assert_eq!(bootstrap_body["bootstrap_required"], false);
        assert_eq!(bootstrap_body["account"]["username"], "parent");
        assert_eq!(bootstrap_body["account"]["display_name"], "Parent Admin");
        assert_eq!(bootstrap_body["cubes"][0]["label"], "T-Cube");
        assert_eq!(bootstrap_body["cubes"][0]["role"], "owner");
        assert!(
            bootstrap_body["cubes"][0]["device_id"]
                .as_str()
                .unwrap()
                .len()
                > 10
        );
        let cookie = bootstrap
            .headers
            .iter()
            .find(|(name, _)| name == "Set-Cookie")
            .map(|(_, value)| value.clone())
            .unwrap();
        assert!(cookie.starts_with("tcube_session="));

        let name_response = route_request(
            &json_request(
                "POST",
                "/api/setup/name",
                json!({ "cube_name": "Nursery Cube" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(name_response.status, 200);
        let name_body: serde_json::Value = serde_json::from_slice(&name_response.body).unwrap();
        let device_id = name_body["device_id"].as_str().unwrap();
        assert_eq!(
            Some(device_id),
            bootstrap_body["cubes"][0]["device_id"].as_str()
        );

        let session_response = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        let session_body: serde_json::Value =
            serde_json::from_slice(&session_response.body).unwrap();
        assert_eq!(session_response.status, 200);
        assert_eq!(session_body["authenticated"], true);
        assert_eq!(session_body["cubes"][0]["device_id"], device_id);
        assert_eq!(session_body["cubes"][0]["role"], "owner");

        let review = setup_review(&config).unwrap();
        assert_eq!(review.cube_name, "Nursery Cube");
        assert_eq!(review.button_modes["1"], "language:English");
        assert_eq!(review.active_counts["language"], 10);
        assert_eq!(review.active_counts["animals"], 10);
        assert_eq!(review.active_counts["music"], 10);

        let wifi_response = route_request(
            &json_request(
                "POST",
                "/api/setup/wifi/verified",
                json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(wifi_response.status, 200);

        let complete_response = route_request(
            &HttpRequest {
                method: "POST".to_string(),
                path: "/api/setup/complete".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(complete_response.status, 200);
        let complete_body: serde_json::Value =
            serde_json::from_slice(&complete_response.body).unwrap();
        assert_eq!(complete_body["status"], "complete");
    }

    #[test]
    fn password_login_sets_session_cookie_and_authenticates_session() {
        let database = test_database();
        let config = test_config(database.path());
        seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let request = json_request(
            "POST",
            "/api/auth/login/password",
            json!({ "username": "admin", "password": "secret-password" }),
            None,
        );

        let response = route_request(&request, &config);

        assert_eq!(response.status, 200);
        let cookie = response
            .headers
            .iter()
            .find(|(name, _)| name == "Set-Cookie")
            .map(|(_, value)| value.clone())
            .unwrap();
        assert!(cookie.starts_with("tcube_session="));

        let session_request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        };
        let session_response = route_request(&session_request, &config);
        let body: serde_json::Value = serde_json::from_slice(&session_response.body).unwrap();

        assert_eq!(session_response.status, 200);
        assert_eq!(body["authenticated"], true);
        assert_eq!(body["account"]["username"], "admin");
    }

    #[test]
    fn single_account_without_membership_is_repaired_as_owner_on_session_read() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "rms", "secret-password").unwrap();
        let conn = Connection::open(database.path()).unwrap();
        conn.execute("delete from cube_memberships", []).unwrap();
        conn.execute("delete from devices", []).unwrap();
        conn.execute("update device_setup set device_id = null where id = 1", [])
            .unwrap();
        let token = create_session(&conn, &account_id).unwrap();
        drop(conn);

        let session = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
                body: Vec::new(),
            },
            &config,
        );

        assert_eq!(session.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&session.body).unwrap();
        assert_eq!(body["authenticated"], true);
        assert_eq!(body["account"]["username"], "rms");
        assert_eq!(body["cubes"][0]["role"], "owner");
        assert!(body["cubes"][0]["device_id"].as_str().unwrap().len() > 10);
        let repaired_count: i64 = Connection::open(database.path())
            .unwrap()
            .query_row("select count(*) from cube_memberships", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(repaired_count, 1);
    }

    #[test]
    fn versioned_admin_api_aliases_support_session_setup_and_events() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let conn = Connection::open(database.path()).unwrap();
        let cookie = session_cookie(&create_session(&conn, &account_id).unwrap());
        conn.execute_batch(
            "create table button_events (
                id integer primary key autoincrement,
                occurred_at text not null,
                button_id integer not null,
                mode text not null,
                response_id text not null,
                response_text text not null
            );",
        )
        .unwrap();
        conn.execute(
            "insert into button_events \
             (occurred_at, button_id, mode, response_id, response_text) \
             values (?1, 1, 'language', 'hello', 'Hello')",
            [now()],
        )
        .unwrap();
        drop(conn);

        let session = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/pi/v1/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(session.status, 200);

        let mode = route_request(
            &json_request(
                "POST",
                "/api/pi/v1/setup/buttons/1/mode",
                json!({ "mode": "language", "language": "French" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(mode.status, 200);

        let events = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/pi/v1/events/recent".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(events.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&events.body).unwrap();
        assert_eq!(body[0]["button_id"], 1);
        assert_eq!(body[0]["response_text"], "Hello");
    }

    #[test]
    fn invalid_password_fails() {
        let database = test_database();
        let config = test_config(database.path());
        seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let request = json_request(
            "POST",
            "/api/auth/login/password",
            json!({ "username": "admin", "password": "wrong-password" }),
            None,
        );

        let response = route_request(&request, &config);

        assert_eq!(response.status, 400);
    }

    #[test]
    fn recovery_code_resets_password_and_revokes_sessions() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "old-password").unwrap();
        let conn = Connection::open(database.path()).unwrap();
        let old_token = create_session(&conn, &account_id).unwrap();
        let recovery_code = "recovery-code";
        conn.execute(
            "insert into recovery_codes (id, account_id, code_hash, created_at, expires_at) \
             values ('recovery-1', ?1, ?2, ?3, ?4)",
            params![
                account_id,
                sha256_hex(recovery_code),
                now(),
                session_expires_at()
            ],
        )
        .unwrap();
        drop(conn);

        let recover = json_request(
            "POST",
            "/api/auth/recover",
            json!({ "code": recovery_code, "password": "new-password" }),
            None,
        );
        let recover_response = route_request(&recover, &config);
        assert_eq!(recover_response.status, 200);

        let old_session = HttpRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), format!("tcube_session={old_token}"))]),
            body: Vec::new(),
        };
        let old_response = route_request(&old_session, &config);
        let old_body: serde_json::Value = serde_json::from_slice(&old_response.body).unwrap();
        assert_eq!(old_body["authenticated"], false);

        let login = json_request(
            "POST",
            "/api/auth/login/password",
            json!({ "username": "admin", "password": "new-password" }),
            None,
        );
        assert_eq!(route_request(&login, &config).status, 200);
    }

    #[test]
    fn authenticated_user_can_create_and_use_recovery_code() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "old-password").unwrap();
        let conn = Connection::open(database.path()).unwrap();
        let token = create_session(&conn, &account_id).unwrap();
        drop(conn);
        let cookie = session_cookie(&token);

        let create_response = route_request(
            &HttpRequest {
                method: "POST".to_string(),
                path: "/api/auth/recovery-code".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(create_response.status, 200);
        let create_body: serde_json::Value = serde_json::from_slice(&create_response.body).unwrap();
        let code = create_body["code"].as_str().unwrap();
        assert!(!code.is_empty());
        assert!(create_body["expires_at"].as_str().unwrap().ends_with('Z'));

        let recover_response = route_request(
            &json_request(
                "POST",
                "/api/auth/recover",
                json!({ "code": code, "password": "new-password" }),
                None,
            ),
            &config,
        );
        assert_eq!(recover_response.status, 200);

        let old_session = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
                body: Vec::new(),
            },
            &config,
        );
        let old_body: serde_json::Value = serde_json::from_slice(&old_session.body).unwrap();
        assert_eq!(old_body["authenticated"], false);

        let login = json_request(
            "POST",
            "/api/auth/login/password",
            json!({ "username": "admin", "password": "new-password" }),
            None,
        );
        assert_eq!(route_request(&login, &config).status, 200);
    }

    #[test]
    fn manager_invitation_can_be_created_and_accepted_once() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );

        let invitation = route_request(
            &json_request(
                "POST",
                "/api/auth/invitations",
                json!({ "device_id": "device-1" }),
                Some(cookie),
            ),
            &config,
        );
        assert_eq!(invitation.status, 200);
        let invitation_body: serde_json::Value = serde_json::from_slice(&invitation.body).unwrap();
        assert_eq!(invitation_body["role"], "manager");
        assert_eq!(invitation_body["device_id"], "device-1");
        let code = invitation_body["code"].as_str().unwrap();

        let accepted = route_request(
            &json_request(
                "POST",
                "/api/auth/invitations/accept",
                json!({
                    "code": code,
                    "username": "manager",
                    "display_name": "Manager",
                    "password": "manager-password"
                }),
                None,
            ),
            &config,
        );
        assert_eq!(accepted.status, 200);
        assert!(accepted
            .headers
            .iter()
            .any(|(name, value)| name == "Set-Cookie" && value.starts_with("tcube_session=")));
        let accepted_body: serde_json::Value = serde_json::from_slice(&accepted.body).unwrap();
        assert_eq!(accepted_body["authenticated"], true);
        assert_eq!(accepted_body["account"]["username"], "manager");
        assert_eq!(accepted_body["cubes"][0]["device_id"], "device-1");
        assert_eq!(accepted_body["cubes"][0]["role"], "manager");

        let second_accept = route_request(
            &json_request(
                "POST",
                "/api/auth/invitations/accept",
                json!({
                    "code": code,
                    "username": "manager-two",
                    "display_name": "Manager Two",
                    "password": "manager-password"
                }),
                None,
            ),
            &config,
        );
        assert_eq!(second_accept.status, 400);
    }

    #[test]
    fn logout_revokes_session() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let conn = Connection::open(database.path()).unwrap();
        let token = create_session(&conn, &account_id).unwrap();
        drop(conn);
        let logout_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/auth/logout".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
            body: Vec::new(),
        };

        let logout_response = route_request(&logout_request, &config);

        assert_eq!(logout_response.status, 200);
        let session_request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
            body: Vec::new(),
        };
        let session_response = route_request(&session_request, &config);
        let body: serde_json::Value = serde_json::from_slice(&session_response.body).unwrap();
        assert_eq!(body["authenticated"], false);
    }

    #[test]
    fn setup_name_and_wifi_mutations_persist() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );

        let name_response = route_request(
            &json_request(
                "POST",
                "/api/setup/name",
                json!({ "cube_name": "Nursery Cube" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(name_response.status, 200);
        let name_body: serde_json::Value = serde_json::from_slice(&name_response.body).unwrap();
        assert_eq!(name_body["name"], "Nursery Cube");
        assert_eq!(name_body["token"], serde_json::Value::Null);

        let wifi_response = route_request(
            &json_request(
                "POST",
                "/api/setup/wifi/verified",
                json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
                Some(cookie),
            ),
            &config,
        );
        assert_eq!(wifi_response.status, 200);

        let review = setup_review(&config).unwrap();
        assert_eq!(review.cube_name, "Nursery Cube");
        assert!(review.wifi_verified);
        assert_eq!(review.dashboard_ip.as_deref(), Some("192.168.50.20"));
    }

    #[test]
    fn button_mode_updates_allow_reused_modes_and_languages() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );

        let response = route_request(
            &json_request(
                "POST",
                "/api/setup/buttons/1/mode",
                json!({ "mode": "language", "language": "Spanish" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(response.status, 200);
        assert_eq!(
            setup_review(&config).unwrap().button_modes["1"],
            "language:Spanish"
        );

        let duplicate = route_request(
            &json_request(
                "POST",
                "/api/setup/buttons/2/mode",
                json!({ "mode": "language", "language": "Spanish" }),
                Some(cookie),
            ),
            &config,
        );
        assert_eq!(duplicate.status, 200);
        let review = setup_review(&config).unwrap();
        assert_eq!(review.button_modes["1"], "language:Spanish");
        assert_eq!(review.button_modes["2"], "language:Spanish");
    }

    #[test]
    fn manager_can_manage_content_but_not_owner_sensitive_actions() {
        let database = test_database();
        let config = test_config(database.path());
        seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let manager_id = seed_manager_account(database.path()).unwrap();
        let conn = Connection::open(database.path()).unwrap();
        let manager_cookie = session_cookie(&create_session(&conn, &manager_id).unwrap());
        seed_active_content(&conn).unwrap();
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values ('manager-draft', 'animals', 2, null, 'Roar', 'Roar', 'data/media/recorded/animals/roar.wav', 'recorded', 'archived', 11)",
            [],
        )
        .unwrap();
        drop(conn);

        let list = route_request(
            &authed_get(
                "/api/content/buttons/2/animals/inactive",
                HashMap::new(),
                &manager_cookie,
            ),
            &config,
        );
        assert_eq!(list.status, 200);

        let mode = route_request(
            &json_request(
                "POST",
                "/api/setup/buttons/4/mode",
                json!({ "mode": "animals" }),
                Some(manager_cookie.clone()),
            ),
            &config,
        );
        assert_eq!(mode.status, 200);

        let activate = route_request(
            &HttpRequest {
                method: "POST".to_string(),
                path: "/api/content/items/manager-draft/activate".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), manager_cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(activate.status, 200);

        let rename = route_request(
            &json_request(
                "POST",
                "/api/setup/name",
                json!({ "cube_name": "Manager Rename" }),
                Some(manager_cookie.clone()),
            ),
            &config,
        );
        assert_eq!(rename.status, 400);

        let wifi = route_request(
            &json_request(
                "POST",
                "/api/setup/wifi/verified",
                json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
                Some(manager_cookie.clone()),
            ),
            &config,
        );
        assert_eq!(wifi.status, 400);

        let invitation = route_request(
            &json_request(
                "POST",
                "/api/auth/invitations",
                json!({ "device_id": "device-1" }),
                Some(manager_cookie.clone()),
            ),
            &config,
        );
        assert_eq!(invitation.status, 400);

        let reset = route_request(
            &json_request(
                "POST",
                "/api/setup/factory-reset",
                json!({ "confirmation": "FACTORY RESET" }),
                Some(manager_cookie),
            ),
            &config,
        );
        assert_eq!(reset.status, 400);
    }

    #[test]
    fn complete_setup_marks_setup_complete_after_prerequisites() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );
        let conn = Connection::open(database.path()).unwrap();
        conn.execute(
            "update device_setup set cube_name = 'Nursery Cube', wifi_verified_at = ?1 where id = 1",
            [now()],
        )
        .unwrap();
        seed_active_content(&conn).unwrap();
        drop(conn);

        let response = route_request(
            &HttpRequest {
                method: "POST".to_string(),
                path: "/api/setup/complete".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );

        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["status"], "complete");
        let complete: i64 = Connection::open(database.path())
            .unwrap()
            .query_row(
                "select setup_complete from device_setup where id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(complete, 1);
    }

    #[test]
    fn content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );
        let conn = Connection::open(database.path()).unwrap();
        seed_active_content(&conn).unwrap();
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values \
             ('generated-draft', 'language', 1, 'English', 'Draft', 'Draft', 'data/audio/draft/language/generated.wav', 'generated', 'archived', 10), \
             ('uploaded-language-draft', 'language', 1, 'English', 'Upload', 'Upload', 'data/audio/draft/language/upload.wav', 'uploaded', 'archived', 11), \
             ('recorded-draft', 'animals', 2, null, 'Roar', 'Roar', 'data/audio/draft/animals/roar.wav', 'recorded', 'archived', 12), \
             ('rejected-draft', 'animals', 2, null, 'Growl', 'Growl', 'data/audio/draft/animals/growl.wav', 'recorded', 'archived', 13)",
            [],
        )
        .unwrap();
        let draft_audio = config.media_root.join("draft/animals/roar.wav");
        fs::create_dir_all(draft_audio.parent().unwrap()).unwrap();
        fs::write(&draft_audio, test_wav()).unwrap();
        let rejected_draft_audio = config.media_root.join("draft/animals/growl.wav");
        fs::write(&rejected_draft_audio, test_wav()).unwrap();
        let generated_draft_audio = config.media_root.join("draft/language/generated.wav");
        fs::create_dir_all(generated_draft_audio.parent().unwrap()).unwrap();
        fs::write(&generated_draft_audio, test_wav()).unwrap();
        let uploaded_language_draft_audio = config.media_root.join("draft/language/upload.wav");
        fs::write(&uploaded_language_draft_audio, test_wav()).unwrap();
        drop(conn);

        let active = route_request(
            &authed_get(
                "/api/content/buttons/1/language/active",
                HashMap::from([("language".to_string(), "English".to_string())]),
                &cookie,
            ),
            &config,
        );
        assert_eq!(active.status, 200);
        let active_body: serde_json::Value = serde_json::from_slice(&active.body).unwrap();
        assert_eq!(active_body["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            active_body["items"][0]["preview_url"],
            serde_json::Value::Null
        );
        assert_eq!(active_body["empty_state"], serde_json::Value::Null);

        let active_language_mismatch = route_request(
            &authed_get(
                "/api/content/buttons/1/language/active",
                HashMap::from([("language".to_string(), "French".to_string())]),
                &cookie,
            ),
            &config,
        );
        assert_eq!(active_language_mismatch.status, 200);
        let active_language_mismatch_body: serde_json::Value =
            serde_json::from_slice(&active_language_mismatch.body).unwrap();
        assert_eq!(
            active_language_mismatch_body["items"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            active_language_mismatch_body["empty_state"]["title"],
            "No active French content on this button"
        );
        assert!(active_language_mismatch_body["empty_state"]["detail"]
            .as_str()
            .unwrap()
            .contains("This button has active content in English"));

        let inactive = route_request(
            &authed_get(
                "/api/content/buttons/2/animals/inactive",
                HashMap::new(),
                &cookie,
            ),
            &config,
        );
        assert_eq!(inactive.status, 200);
        let inactive_body: serde_json::Value = serde_json::from_slice(&inactive.body).unwrap();
        assert_eq!(inactive_body["items"][0]["id"], "recorded-draft");
        assert_eq!(
            inactive_body["items"][0]["preview_url"],
            "/api/media/draft/animals/roar.wav"
        );
        assert_eq!(inactive_body["empty_state"], serde_json::Value::Null);

        let inactive_language_mismatch = route_request(
            &authed_get(
                "/api/content/buttons/1/language/inactive",
                HashMap::from([("language".to_string(), "French".to_string())]),
                &cookie,
            ),
            &config,
        );
        assert_eq!(inactive_language_mismatch.status, 200);
        let inactive_language_mismatch_body: serde_json::Value =
            serde_json::from_slice(&inactive_language_mismatch.body).unwrap();
        assert_eq!(
            inactive_language_mismatch_body["items"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            inactive_language_mismatch_body["empty_state"]["title"],
            "No draft French content on this button"
        );
        assert!(inactive_language_mismatch_body["empty_state"]["detail"]
            .as_str()
            .unwrap()
            .contains("This button has draft content in English"));

        let activate = route_request(
            &HttpRequest {
                method: "POST".to_string(),
                path: "/api/content/items/recorded-draft/activate".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(activate.status, 200);
        let (activated_state, activated_audio_path): (String, String) =
            Connection::open(database.path())
                .unwrap()
                .query_row(
                    "select state, audio_path from content_items where id = 'recorded-draft'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
        assert_eq!(activated_state, "active");
        assert_eq!(activated_audio_path, "data/audio/active/animals/roar.wav");
        assert!(!draft_audio.exists());
        assert!(config.media_root.join("active/animals/roar.wav").exists());

        let cleanup = route_request(
            &json_request(
                "DELETE",
                "/api/content/generated-speech/unused",
                json!({ "button_id": 1, "language": "English" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(cleanup.status, 200);
        let cleanup_body: serde_json::Value = serde_json::from_slice(&cleanup.body).unwrap();
        assert_eq!(cleanup_body["deleted_count"], 2);
        assert!(!generated_draft_audio.exists());
        assert!(!uploaded_language_draft_audio.exists());

        let reject = route_request(
            &HttpRequest {
                method: "DELETE".to_string(),
                path: "/api/content/items/rejected-draft".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(reject.status, 200);
        assert!(!rejected_draft_audio.exists());

        let trash = route_request(
            &HttpRequest {
                method: "DELETE".to_string(),
                path: "/api/content/items/language-one".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(trash.status, 200);
        let trashed_state: String = Connection::open(database.path())
            .unwrap()
            .query_row(
                "select state from content_items where id = 'language-one'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trashed_state, "trash");
    }

    #[test]
    fn content_inventory_classifies_current_drafts_and_unused_audio() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );
        let conn = Connection::open(database.path()).unwrap();
        seed_active_content(&conn).unwrap();
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values \
             ('inventory-active-one', 'language', 1, 'English', 'Hello audio', 'Hello audio', 'data/audio/active/language/hello.wav', 'recorded', 'active', 20), \
             ('inventory-active-two', 'language', 1, 'English', 'Bye audio', 'Bye audio', 'data/audio/active/language/bye.wav', 'recorded', 'active', 21), \
             ('inventory-draft', 'language', 1, 'French', 'Bonjour', 'Bonjour', 'data/audio/draft/language/bonjour.wav', 'recorded', 'archived', 22)",
            [],
        )
        .unwrap();
        drop(conn);
        fs::create_dir_all(config.media_root.join("active/language")).unwrap();
        let hello_audio = config.media_root.join("active/language/hello.wav");
        let bye_audio = config.media_root.join("active/language/bye.wav");
        fs::write(&hello_audio, b"hello").unwrap();
        fs::write(&bye_audio, b"bye").unwrap();

        let mode = route_request(
            &json_request(
                "POST",
                "/api/setup/buttons/1/mode",
                json!({ "mode": "language", "language": "French" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(mode.status, 200);

        let inventory = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/content/inventory".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(inventory.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&inventory.body).unwrap();
        assert_eq!(body["draft_count"], 1);
        assert_eq!(body["unused_count"], 2);
        assert_eq!(body["active_count"], 0);
        let draft = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "inventory-draft")
            .unwrap();
        assert_eq!(draft["status"], "draft");
        let unused = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "inventory-active-one")
            .unwrap();
        assert_eq!(unused["status"], "unused");
        assert!(unused["reason"]
            .as_str()
            .unwrap()
            .contains("Button 1 is set to French"));

        let cleanup = route_request(
            &HttpRequest {
                method: "DELETE".to_string(),
                path: "/api/content/unused".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(cleanup.status, 200);
        let cleanup_body: serde_json::Value = serde_json::from_slice(&cleanup.body).unwrap();
        assert_eq!(cleanup_body["deleted_count"], 2);
        assert!(!hello_audio.exists());
        assert!(!bye_audio.exists());
        let trashed_count: i64 = Connection::open(database.path())
            .unwrap()
            .query_row(
                "select count(*) from content_items where id in ('inventory-active-one', 'inventory-active-two') and state = 'trash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trashed_count, 2);
    }

    #[test]
    fn factory_reset_requires_confirmation_and_restores_first_run_defaults() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let token =
            create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap();
        let cookie = session_cookie(&token);

        let wrong_confirmation = route_request(
            &json_request(
                "POST",
                "/api/pi/v1/setup/factory-reset",
                json!({ "confirmation": "reset" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(wrong_confirmation.status, 400);

        let session_before = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(session_before.status, 200);

        let conn = Connection::open(database.path()).unwrap();
        migrate_admin_database(&conn, &config).unwrap();
        conn.execute(
            "update device_setup \
             set setup_complete = 1, cube_name = 'Nursery Cube', wifi_ssid = 'Home', \
                 wifi_verified_at = ?1, dashboard_ip = '192.168.50.20', updated_at = ?1 \
             where id = 1",
            [now()],
        )
        .unwrap();
        conn.execute(
            "insert into trusted_sessions (id, label, created_at) values ('trusted-1', 'Phone', ?1)",
            [now()],
        )
        .unwrap();
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values \
             ('reset-active', 'language', 1, 'English', 'Parent active', 'Parent active', 'data/audio/active/language/parent.wav', 'recorded', 'active', 50), \
             ('reset-draft', 'language', 1, 'English', 'Parent draft', 'Parent draft', 'data/audio/draft/language/draft.wav', 'uploaded', 'archived', 51)",
            [],
        )
        .unwrap();
        conn.execute(
            "insert into media_artifacts (id, content_item_id, media_type, path, state) \
             values ('artifact-1', 'reset-active', 'recorded_audio', 'data/audio/active/language/parent.wav', 'active')",
            [],
        )
        .unwrap();
        conn.execute(
            "insert into content_jobs (id, job_type, status, language) \
             values ('job-1', 'tts', 'queued', 'English')",
            [],
        )
        .unwrap();
        conn.execute(
            "insert into button_events (occurred_at, button_id, mode, response_id, response_text) \
             values (?1, 1, 'language', 'reset-active', 'Parent active')",
            [now()],
        )
        .unwrap();
        conn.execute(
            "insert into setup_debug_events (event_type, button_id, details) \
             values ('setup_help_button_press', 4, '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "insert into content_packages \
             (package_id, device_id, revision, schema_version, minimum_runtime_version, status, created_at) \
             values ('package-1', 'device-1', 1, 1, '0.0.1', 'active', ?1)",
            [now()],
        )
        .unwrap();
        conn.execute(
            "insert into content_package_failures \
             (device_id, package_id, runtime_version, stage, detail, occurred_at) \
             values ('device-1', 'package-1', '0.0.1', 'download', 'failed', ?1)",
            [now()],
        )
        .unwrap();
        drop(conn);

        let active_audio = config.media_root.join("active/language/parent.wav");
        let draft_audio = config.media_root.join("draft/language/draft.wav");
        fs::create_dir_all(active_audio.parent().unwrap()).unwrap();
        fs::create_dir_all(draft_audio.parent().unwrap()).unwrap();
        fs::write(&active_audio, b"active").unwrap();
        fs::write(&draft_audio, b"draft").unwrap();

        let reset = route_request(
            &json_request(
                "POST",
                "/api/pi/v1/setup/factory-reset",
                json!({ "confirmation": "FACTORY RESET" }),
                Some(cookie.clone()),
            ),
            &config,
        );
        assert_eq!(reset.status, 200);
        assert!(reset
            .headers
            .iter()
            .any(|(name, value)| name == "Set-Cookie" && value.starts_with("tcube_session=;")));
        let reset_body: serde_json::Value = serde_json::from_slice(&reset.body).unwrap();
        assert_eq!(reset_body["bootstrap_required"], true);
        assert!(!active_audio.exists());
        assert!(!draft_audio.exists());

        let conn = Connection::open(database.path()).unwrap();
        assert_eq!(table_count(&conn, "admin_accounts").unwrap(), 0);
        assert_eq!(table_count(&conn, "admin_sessions").unwrap(), 0);
        assert_eq!(table_count(&conn, "button_events").unwrap(), 0);
        assert_eq!(table_count(&conn, "setup_debug_events").unwrap(), 0);
        assert_eq!(table_count(&conn, "content_jobs").unwrap(), 0);
        assert_eq!(table_count(&conn, "media_artifacts").unwrap(), 0);
        assert_eq!(table_count(&conn, "content_packages").unwrap(), 0);
        assert_eq!(table_count(&conn, "content_package_failures").unwrap(), 0);
        assert_eq!(table_count(&conn, "button_mappings").unwrap(), 5);
        assert_eq!(table_count(&conn, "content_items").unwrap(), 30);
        let setup_complete: i64 = conn
            .query_row(
                "select setup_complete from device_setup where id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(setup_complete, 0);
        let top_mode: String = conn
            .query_row(
                "select mode || ':' || coalesce(language, '') from button_mappings where button_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(top_mode, "language:English");
        drop(conn);

        let session_after = route_request(
            &HttpRequest {
                method: "GET".to_string(),
                path: "/api/auth/session".to_string(),
                query: HashMap::new(),
                headers: HashMap::from([("cookie".to_string(), cookie)]),
                body: Vec::new(),
            },
            &config,
        );
        assert_eq!(session_after.status, 200);
        let session_after_body: serde_json::Value =
            serde_json::from_slice(&session_after.body).unwrap();
        assert_eq!(session_after_body["authenticated"], false);
        assert_eq!(session_after_body["bootstrap_required"], true);

        let rebootstrap = route_request(
            &json_request(
                "POST",
                "/api/auth/bootstrap",
                json!({
                    "username": "rms",
                    "display_name": "rms",
                    "password": "owner-password"
                }),
                None,
            ),
            &config,
        );
        assert_eq!(rebootstrap.status, 200);
        let rebootstrap_body: serde_json::Value =
            serde_json::from_slice(&rebootstrap.body).unwrap();
        assert_eq!(rebootstrap_body["authenticated"], true);
        assert_eq!(rebootstrap_body["account"]["username"], "rms");
        assert_eq!(rebootstrap_body["cubes"][0]["role"], "owner");
        assert!(
            rebootstrap_body["cubes"][0]["device_id"]
                .as_str()
                .unwrap()
                .len()
                > 10
        );
    }

    #[test]
    fn multipart_recording_and_upload_create_inactive_drafts() {
        let database = test_database();
        let config = test_config(database.path());
        let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
        let cookie = session_cookie(
            &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
        );
        let wav = test_wav();

        let recorded = route_request(
            &multipart_request(
                "/api/content/recordings",
                &cookie,
                vec![
                    ("content_type", "language"),
                    ("button_id", "1"),
                    ("title", ""),
                    ("text", "Hello baby"),
                    ("language", "English"),
                ],
                "audio_file",
                "language-recording.wav",
                "audio/wav",
                wav.clone(),
            ),
            &config,
        );
        assert_eq!(recorded.status, 200);
        let recorded_body: serde_json::Value = serde_json::from_slice(&recorded.body).unwrap();
        assert_eq!(recorded_body["state"], "archived");
        assert_eq!(recorded_body["source"], "recorded");
        assert_eq!(recorded_body["text"], "Hello baby");
        assert!(recorded_body["title"]
            .as_str()
            .unwrap()
            .starts_with("recorded-english-hello-baby-"));
        assert!(recorded_body["audio_path"]
            .as_str()
            .unwrap()
            .starts_with("data/audio/draft/language/recorded-english-hello-baby-"));
        let recorded_path = recorded_body["audio_path"]
            .as_str()
            .unwrap()
            .strip_prefix("data/audio/")
            .unwrap();
        assert!(config.media_root.join(recorded_path).exists());

        let recorded_without_title = route_request(
            &multipart_request(
                "/api/content/recordings",
                &cookie,
                vec![
                    ("content_type", "language"),
                    ("button_id", "4"),
                    ("title", ""),
                    ("text", "Bonjour bebe"),
                    ("language", "French"),
                ],
                "audio_file",
                "french-recording.wav",
                "audio/wav",
                wav.clone(),
            ),
            &config,
        );
        assert_eq!(recorded_without_title.status, 200);
        let recorded_without_title_body: serde_json::Value =
            serde_json::from_slice(&recorded_without_title.body).unwrap();
        assert_eq!(recorded_without_title_body["state"], "archived");
        assert_eq!(recorded_without_title_body["text"], "Bonjour bebe");
        assert_eq!(recorded_without_title_body["language"], "French");
        assert!(recorded_without_title_body["title"]
            .as_str()
            .unwrap()
            .starts_with("recorded-french-bonjour-bebe-"));
        let french_recorded_path = recorded_without_title_body["audio_path"]
            .as_str()
            .unwrap()
            .strip_prefix("data/audio/")
            .unwrap();
        assert!(config.media_root.join(french_recorded_path).exists());

        let recorded_without_spoken_text = route_request(
            &multipart_request(
                "/api/content/recordings",
                &cookie,
                vec![
                    ("content_type", "language"),
                    ("button_id", "4"),
                    ("title", ""),
                    ("text", ""),
                    ("language", "French"),
                ],
                "audio_file",
                "french-recording.wav",
                "audio/wav",
                wav.clone(),
            ),
            &config,
        );
        assert_eq!(recorded_without_spoken_text.status, 400);

        let uploaded = route_request(
            &multipart_request(
                "/api/content/uploads",
                &cookie,
                vec![
                    ("content_type", "animals"),
                    ("button_id", "2"),
                    ("title", "Roar"),
                    ("text", ""),
                    ("language", ""),
                ],
                "audio_file",
                "roar.wav",
                "audio/wav",
                wav,
            ),
            &config,
        );
        assert_eq!(uploaded.status, 200);
        let uploaded_body: serde_json::Value = serde_json::from_slice(&uploaded.body).unwrap();
        assert_eq!(uploaded_body["source"], "uploaded");
        assert_eq!(uploaded_body["title"], "Roar");
        assert_eq!(uploaded_body["text"], "Roar");
        assert!(uploaded_body["preview_url"]
            .as_str()
            .unwrap()
            .starts_with("/api/media/draft/animals/"));
        let uploaded_path = uploaded_body["audio_path"]
            .as_str()
            .unwrap()
            .strip_prefix("data/audio/")
            .unwrap();
        assert!(config.media_root.join(uploaded_path).exists());
    }

    struct TestDatabase {
        _dir: TempDir,
        path: PathBuf,
    }

    impl TestDatabase {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    fn test_database() -> TestDatabase {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tcube.sqlite3");
        TestDatabase { _dir: dir, path }
    }

    fn test_config(database: &Path) -> AdminConfig {
        AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: database.to_path_buf(),
            ui_dist: PathBuf::from("admin-ui"),
            media_root: database.parent().unwrap().join("media"),
            content_root: database.parent().unwrap().join("content"),
            hostname: "tcube-a7f3.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
        }
    }

    #[test]
    fn speech_http_client_loads_custom_root_certificate() {
        let temp_dir = TempDir::new().unwrap();
        let cert_path = temp_dir.path().join("speech-api-ca.crt");
        fs::write(
            &cert_path,
            b"-----BEGIN CERTIFICATE-----\nMIICpDCCAYwCCQDpAesS5Rc0YzANBgkqhkiG9w0BAQsFADAUMRIwEAYDVQQDDAls\nb2NhbGhvc3QwHhcNMjYwNjIyMTEyMzMyWhcNMjYwNjIzMTEyMzMyWjAUMRIwEAYD\nVQQDDAlsb2NhbGhvc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDZ\nYlRHQ24BleueDVCphzdU7ONSyLlcrR4cDlQp9ayS6z4R3ORxz18FVdABXBzBlOT6\njNLRacsgTZLOra4r+eQclls8PWj6OWkq6jFfjzYJI13rjJEwdX+k49i2riUgS3n3\nwSr7LIn56moi2r8AmGD7mZKijNXODAQ+rIT8DKKpiw7igbghUsHhD5LOZMiqNGoB\n1XGFZmYPq0F1E1rNVzpl2PEVBWxUNk9DiQPvUGNGwlcfBEniH5dfCuDfAUYeHBLY\nIPT69KoSeCoBShSvMGgewIQz16+783QAOzmC5brAZgrlKeCCNFx7QjrTouWZ1MK0\nMs+YcoQFHoEgenCs9RnZAgMBAAEwDQYJKoZIhvcNAQELBQADggEBABX4bq6VntHb\n0y52sA8w11qMR81S5IemcDzQhdwBN7Oe8Sdg3pu1xM+BuMxfmbYVP20Lt1SKIm96\n5Yuq8vjhYtvYHDFU5qkTg5vmyrJ0C+HZSlDSzGYHTKuS1tjmTOpZkUZU+SM3bXXi\nmgqwVxJ9W0dCKyKJaI5A0uPbwuGkwmOxPMoy+pqPeDY+tHrJ/bp66ew/4K2g4SDz\n/tyIpaKcKngpaVxmrml7pZ11CobuuPznIL9EGkzJQ3VRFs6CmKAbkV5X1Fx6Q1Ok\ntpfBGYghLsPt5k32bp/4+oaxGOBEV5DNSSKb8MA+dvmwWJXq0QW8G56fHlsI1q9b\nWrcxxfJMPHk=\n-----END CERTIFICATE-----\n",
        )
        .unwrap();

        let client = speech_http_client_with_ca_cert_path(Some(&cert_path)).unwrap();
        let response = client.get("https://example.com").build();
        assert!(response.is_ok());
    }

    #[test]
    fn speech_provider_urls_must_use_https() {
        let secure = validate_speech_api_url("https://localhost:8001/v1").unwrap();
        assert_eq!(secure.scheme(), "https");

        let insecure = validate_speech_api_url("http://localhost:8001/v1").unwrap_err();
        assert!(insecure
            .to_string()
            .contains("speech provider URL must use https"));
    }

    #[test]
    fn speech_provider_probe_rejects_insecure_urls() {
        let error = probe_speech_provider("http://localhost:8001/v1").unwrap_err();

        assert!(error
            .to_string()
            .contains("speech provider URL must use https"));
    }

    #[test]
    fn speech_provider_health_cache_reuses_recent_result() {
        let probe_count = AtomicUsize::new(0);
        let key = format!("test-provider:{}", random_token(8).unwrap());

        let first = cached_speech_provider_health(key.clone(), "voxtral".to_string(), || {
            probe_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
        let second = cached_speech_provider_health(key, "voxtral".to_string(), || {
            probe_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();

        assert!(first.0.online);
        assert!(!first.1);
        assert!(second.0.online);
        assert!(second.1);
        assert_eq!(probe_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn generated_language_filename_includes_model_language_and_text() {
        let filename = generated_filename("voxtral", "French", "Bonjour bebe", "wav");

        assert!(filename.starts_with("generated-voxtral-french-bonjour-bebe-"));
        assert!(filename.ends_with(".wav"));
    }

    fn json_request(
        method: &str,
        path: &str,
        body: serde_json::Value,
        cookie: Option<String>,
    ) -> HttpRequest {
        let body = serde_json::to_vec(&body).unwrap();
        let mut headers = HashMap::from([("content-length".to_string(), body.len().to_string())]);
        if let Some(cookie) = cookie {
            headers.insert("cookie".to_string(), cookie);
        }
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            query: HashMap::new(),
            headers,
            body,
        }
    }

    fn authed_get(path: &str, query: HashMap<String, String>, cookie: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".to_string(),
            path: path.to_string(),
            query,
            headers: HashMap::from([("cookie".to_string(), cookie.to_string())]),
            body: Vec::new(),
        }
    }

    fn seed_auth_database(path: &Path, username: &str, password: &str) -> Result<String> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "create table admin_accounts (
                id text primary key,
                username text not null unique collate nocase,
                display_name text not null,
                password_hash text,
                created_at text not null,
                disabled_at text
            );
            create table devices (
                id text primary key,
                label text not null,
                token_hash text not null,
                created_at text not null,
                last_seen_at text,
                revoked_at text
            );
            create table cube_memberships (
                account_id text not null,
                device_id text not null,
                role text not null check (role in ('owner', 'manager')),
                created_at text not null,
                primary key (account_id, device_id)
            );
            create table admin_sessions (
                id text primary key,
                account_id text not null,
                token_hash text not null unique,
                created_at text not null,
                last_seen_at text not null,
                expires_at text not null,
                revoked_at text
            );
            create table cube_invitations (
                id text primary key,
                device_id text not null,
                invited_by text not null,
                role text not null check (role = 'manager'),
                code_hash text not null unique,
                created_at text not null,
                expires_at text not null,
                accepted_at text,
                accepted_by text,
                revoked_at text
            );
            create table recovery_codes (
                id text primary key,
                account_id text not null,
                code_hash text not null unique,
                created_at text not null,
                expires_at text not null,
                used_at text
            );
            create table device_setup (
                id integer primary key check (id = 1),
                setup_complete integer not null default 0,
                cube_name text,
                device_id text,
                admin_credential_hash text,
                wifi_ssid text,
                wifi_verified_at text,
                dashboard_host text not null default 'tcube.local',
                dashboard_ip text,
                updated_at text not null default current_timestamp
            );
            create table button_mappings (
                button_id integer primary key check (button_id between 1 and 5),
                mode text not null,
                language text,
                content_type text,
                manual_order_weight integer not null default 0,
                updated_at text not null default current_timestamp
            );
            create table content_items (
                id text primary key,
                content_type text not null,
                button_id integer,
                language text,
                title text,
                text text,
                audio_path text,
                source text not null,
                state text not null default 'active',
                order_index integer not null default 0,
                created_at text not null default current_timestamp,
                updated_at text not null default current_timestamp,
                trashed_at text,
                purge_after text
            );
            create table media_artifacts (
                id text primary key,
                content_item_id text,
                media_type text not null,
                path text,
                text text,
                state text not null default 'active'
            );",
        )?;
        let account_id = "account-1".to_string();
        conn.execute(
            "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
             values (?1, ?2, 'Local owner', ?3, ?4)",
            params![account_id, username, hash_password(password)?, now()],
        )?;
        conn.execute(
            "insert into devices (id, label, token_hash, created_at) values ('device-1', 'T-Cube', 'hash', ?1)",
            [now()],
        )?;
        conn.execute(
            "insert into cube_memberships (account_id, device_id, role, created_at) values (?1, 'device-1', 'owner', ?2)",
            params![account_id, now()],
        )?;
        conn.execute(
            "insert into device_setup (id, device_id, dashboard_host) values (1, 'device-1', 'tcube.local')",
            [],
        )?;
        conn.execute(
            "insert into button_mappings (button_id, mode, language, content_type, manual_order_weight) values
             (1, 'language', 'English', 'language', 0),
             (2, 'animals', null, 'animals', 1),
             (3, 'music', null, 'music', 2)",
            [],
        )?;
        Ok(account_id)
    }

    fn seed_manager_account(path: &Path) -> Result<String> {
        let conn = Connection::open(path)?;
        let account_id = "manager-account".to_string();
        conn.execute(
            "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
             values (?1, 'manager', 'Local manager', ?2, ?3)",
            params![account_id, hash_password("manager-password")?, now()],
        )?;
        add_cube_membership(&conn, &account_id, "device-1", CubeRole::Manager)?;
        Ok(account_id)
    }

    fn seed_active_content(conn: &Connection) -> Result<()> {
        conn.execute(
            "insert into content_items (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) values
             ('language-one', 'language', 1, 'English', 'Hello', 'Hello', null, 'default', 'active', 0),
             ('animal-one', 'animals', 2, null, 'Moo', 'Moo', null, 'default', 'active', 0),
             ('music-one', 'music', 3, null, 'Song', 'Song', null, 'default', 'active', 0)",
            [],
        )?;
        Ok(())
    }

    fn multipart_request(
        path: &str,
        cookie: &str,
        fields: Vec<(&str, &str)>,
        file_field: &str,
        filename: &str,
        content_type: &str,
        file_bytes: Vec<u8>,
    ) -> HttpRequest {
        let boundary = "tcube-test-boundary";
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                    .as_bytes(),
            );
        }
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{file_field}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&file_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        HttpRequest {
            method: "POST".to_string(),
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::from([
                ("cookie".to_string(), cookie.to_string()),
                (
                    "content-type".to_string(),
                    format!("multipart/form-data; boundary={boundary}"),
                ),
                ("content-length".to_string(), body.len().to_string()),
            ]),
            body,
        }
    }

    fn test_wav() -> Vec<u8> {
        let sample_rate = 8_000_u32;
        let samples = 800_u32;
        let data_size = samples * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for index in 0..samples {
            let value = if index % 2 == 0 {
                10_000_i16
            } else {
                -10_000_i16
            };
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
