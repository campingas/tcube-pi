use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::config::AdminConfig;
use crate::db::admin::auth::{authenticate_session, require_local_cube_role, RoleRequirement};
use crate::db::admin::pomodoro as pomodoro_storage;
use crate::db::admin::schema::open_existing_database;
use crate::db::admin::setup::{self as setup_storage, SetupReview};

pub(crate) use crate::db::admin::pomodoro::{
    PomodoroSettingsUpdate, PomodoroSettingsWithRecommendation,
};

const FACTORY_RESET_CONFIRMATION: &str = "FACTORY RESET";

#[derive(Debug, Serialize)]
pub(crate) struct SetupReviewResponse {
    pub(crate) cube_name: String,
    pub(crate) device_id: Option<String>,
    pub(crate) admin_created: bool,
    pub(crate) wifi_verified: bool,
    pub(crate) wifi_ssid: Option<String>,
    pub(crate) dashboard_ip: Option<String>,
    pub(crate) dashboard_address: String,
    pub(crate) button_modes: HashMap<String, String>,
    pub(crate) active_counts: HashMap<String, i64>,
}

impl From<SetupReview> for SetupReviewResponse {
    fn from(review: SetupReview) -> Self {
        Self {
            cube_name: review.cube_name,
            device_id: review.device_id,
            admin_created: review.admin_created,
            wifi_verified: review.wifi_verified,
            wifi_ssid: review.wifi_ssid,
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

pub(crate) fn setup_review(config: &AdminConfig) -> Result<SetupReviewResponse> {
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok(setup_storage::default_setup_review(config).into());
    };
    setup_review_from_conn(config, &conn)
}

pub(crate) fn pomodoro_settings(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<PomodoroSettingsWithRecommendation> {
    let conn = authenticated_connection(config, token)?;
    pomodoro_storage::get_settings(&conn)
}

pub(crate) fn save_pomodoro_settings(
    config: &AdminConfig,
    token: Option<&str>,
    body: PomodoroSettingsUpdate,
) -> Result<PomodoroSettingsWithRecommendation> {
    let conn = owner_connection(config, token)?;
    pomodoro_storage::save_settings(&conn, body)
}

pub(crate) fn set_cube_name(
    config: &AdminConfig,
    token: Option<&str>,
    body: NameRequest,
) -> Result<CubeSaveResponse> {
    let conn = owner_connection(config, token)?;
    let session = authenticate_session(&conn, token)?.context("authentication required")?;
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

pub(crate) fn verify_wifi(
    config: &AdminConfig,
    token: Option<&str>,
    body: WifiRequest,
) -> Result<()> {
    let conn = owner_connection(config, token)?;
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
    token: Option<&str>,
    button_id: i64,
    body: ButtonModeRequest,
) -> Result<()> {
    let conn = authenticated_connection(config, token)?;
    if !(1..=5).contains(&button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
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
    token: Option<&str>,
) -> Result<CompleteSetupResponse> {
    let conn = owner_connection(config, token)?;
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
    token: Option<&str>,
    body: FactoryResetRequest,
) -> Result<FactoryResetResponse> {
    let mut conn = owner_connection(config, token)?;
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

fn setup_review_from_conn(config: &AdminConfig, conn: &Connection) -> Result<SetupReviewResponse> {
    Ok(setup_storage::setup_review_from_conn(config, conn)?.into())
}

fn authenticated_connection(config: &AdminConfig, token: Option<&str>) -> Result<Connection> {
    role_authorized_connection(config, token, RoleRequirement::Member)
}

fn owner_connection(config: &AdminConfig, token: Option<&str>) -> Result<Connection> {
    role_authorized_connection(config, token, RoleRequirement::Owner)
}

fn role_authorized_connection(
    config: &AdminConfig,
    token: Option<&str>,
    requirement: RoleRequirement,
) -> Result<Connection> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    require_local_cube_role(&conn, &session.account.id, requirement)?;
    Ok(conn)
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
