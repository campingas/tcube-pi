use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::config::AdminConfig;
use crate::db::admin::auth::{authenticate_session, require_local_cube_role, RoleRequirement};

/// Number of trailing log lines surfaced to the admin UI.
const LOG_TAIL_LINES: usize = 3;
/// Timeout for the GitHub releases API lookup.
const GITHUB_API_TIMEOUT: Duration = Duration::from_secs(10);
/// The updater unit has a 30-minute timeout. Allow five minutes for its failure
/// trap and systemd cleanup before classifying an orphaned running marker.
const RUNNING_STALE_AFTER: chrono::Duration = chrono::Duration::minutes(35);

const STATE_IDLE: &str = "idle";
const STATE_QUEUED: &str = "queued";
const STATE_RUNNING: &str = "running";
const STATE_SUCCESS: &str = "success";
const STATE_FAILED: &str = "failed";

#[derive(Debug, thiserror::Error)]
#[error("an update is already running")]
pub(crate) struct UpdateAlreadyRunning;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct UpdateStatusResponse {
    installed_version: Option<String>,
    state: String,
    log_lines: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct UpdateCheckResponse {
    installed_version: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateInstallResponse {
    accepted: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct StoredState {
    state: String,
    started_at: Option<DateTime<Utc>>,
}

/// Installed version plus the state and tail of any in-progress or finished update.
pub(crate) fn update_status(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<UpdateStatusResponse> {
    authorize(config, token, RoleRequirement::Member)?;
    Ok(read_update_status(config, Utc::now()))
}

/// Compares the installed version against the latest stable GitHub release.
pub(crate) fn check_update(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<UpdateCheckResponse> {
    authorize(config, token, RoleRequirement::Owner)?;
    let installed_version = read_installed_version(&config.version_file);
    let latest_version = fetch_latest_release_tag(&config.update_repo)?;
    let update_available =
        compute_update_available(latest_version.as_deref(), installed_version.as_deref());
    Ok(UpdateCheckResponse {
        installed_version,
        latest_version,
        update_available,
    })
}

/// Exclusively creates the one request file watched by the privileged updater.
/// The installer owns the directory topology; this unprivileged API never creates it.
pub(crate) fn request_install(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<UpdateInstallResponse> {
    authorize(config, token, RoleRequirement::Owner)?;
    let stored = read_stored_state(&config.update_dir);
    if running_is_live(&stored, Utc::now()) {
        return Err(UpdateAlreadyRunning.into());
    }

    let request = request_path(&config.update_dir);
    if request.is_file() && !request.is_symlink() {
        return Ok(UpdateInstallResponse { accepted: true });
    }
    if request.exists() || request.is_symlink() {
        anyhow::bail!(
            "update request path is not a regular file: {}",
            request.display()
        );
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&request)
    {
        Ok(mut file) => file
            .write_all(b"install\n")
            .with_context(|| format!("failed to write update request {}", request.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !request.is_file() || request.is_symlink() {
                anyhow::bail!(
                    "update request path is not a regular file: {}",
                    request.display()
                );
            }
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to create update request {}; reinstall the current release to repair the updater layout",
                    request.display()
                )
            });
        }
    }
    Ok(UpdateInstallResponse { accepted: true })
}

fn authorize(
    config: &AdminConfig,
    token: Option<&str>,
    requirement: RoleRequirement,
) -> Result<()> {
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
    Ok(())
}

fn fetch_latest_release_tag(repo: &str) -> Result<Option<String>> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(GITHUB_API_TIMEOUT)
        .build()
        .context("failed to build update HTTP client")?;
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "tcube-pi-admin")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .context("failed to reach the GitHub releases API")?;
    if !response.status().is_success() {
        anyhow::bail!("GitHub releases API returned {}", response.status());
    }
    let raw = response
        .text()
        .context("failed to read the GitHub releases response")?;
    let body: serde_json::Value =
        serde_json::from_str(&raw).context("failed to parse the GitHub releases response")?;
    Ok(body
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn compute_update_available(latest: Option<&str>, installed: Option<&str>) -> bool {
    match latest {
        Some(latest) => !latest.is_empty() && Some(latest) != installed,
        None => false,
    }
}

fn read_update_status(config: &AdminConfig, now: DateTime<Utc>) -> UpdateStatusResponse {
    let installed_version = read_installed_version(&config.version_file);
    let stored = read_stored_state(&config.update_dir);
    let request_queued = request_path(&config.update_dir).is_file();
    let (state, stale_error) = effective_state(&stored, request_queued, now);
    let log_lines = read_log_tail(&config.update_dir, LOG_TAIL_LINES);
    let error = if let Some(error) = stale_error {
        Some(error)
    } else if state == STATE_FAILED {
        log_lines.last().cloned()
    } else {
        None
    };
    UpdateStatusResponse {
        installed_version,
        state,
        log_lines,
        error,
    }
}

fn effective_state(
    stored: &StoredState,
    request_queued: bool,
    now: DateTime<Utc>,
) -> (String, Option<String>) {
    if running_is_live(stored, now) {
        return (STATE_RUNNING.to_string(), None);
    }
    if request_queued {
        return (STATE_QUEUED.to_string(), None);
    }
    if stored.state == STATE_RUNNING {
        let error = match stored.started_at {
            Some(started_at) => format!(
                "Update has been marked running since {}; it exceeded the 35-minute safety window. Check tcube-update.service.",
                started_at.to_rfc3339()
            ),
            None => "Update was marked running without a start time and may have been interrupted. Check tcube-update.service.".to_string(),
        };
        return (STATE_FAILED.to_string(), Some(error));
    }
    let state = match stored.state.as_str() {
        STATE_SUCCESS => STATE_SUCCESS,
        STATE_FAILED => STATE_FAILED,
        _ => STATE_IDLE,
    };
    (state.to_string(), None)
}

fn running_is_live(stored: &StoredState, now: DateTime<Utc>) -> bool {
    stored.state == STATE_RUNNING
        && stored.started_at.is_some_and(|started_at| {
            let age = now.signed_duration_since(started_at);
            age >= chrono::Duration::zero() && age <= RUNNING_STALE_AFTER
        })
}

fn read_installed_version(path: &Path) -> Option<String> {
    first_nonempty_line(&fs::read_to_string(path).ok()?)
}

fn read_stored_state(dir: &Path) -> StoredState {
    let Ok(content) = fs::read_to_string(dir.join("state")) else {
        return StoredState::default();
    };
    let mut stored = StoredState::default();
    for line in content.lines().map(str::trim) {
        let Some((key, value)) = line.split_once('=') else {
            if stored.state.is_empty() {
                stored.state = line.to_string();
            }
            continue;
        };
        match key {
            "state" => stored.state = value.trim().to_string(),
            "started_at" => {
                stored.started_at = DateTime::parse_from_rfc3339(value.trim())
                    .ok()
                    .map(|value| value.with_timezone(&Utc));
            }
            _ => {}
        }
    }
    stored
}

fn read_log_tail(dir: &Path, count: usize) -> Vec<String> {
    let Ok(content) = fs::read_to_string(dir.join("log")) else {
        return Vec::new();
    };
    let lines: Vec<String> = content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].to_vec()
}

fn first_nonempty_line(content: &str) -> Option<String> {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn request_path(update_dir: &Path) -> std::path::PathBuf {
    update_dir.join("requests/install")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    use crate::config::AdminConfig;

    use super::{
        compute_update_available, read_installed_version, read_log_tail, read_update_status,
        STATE_FAILED, STATE_IDLE, STATE_QUEUED, STATE_RUNNING,
    };

    fn config(dir: &TempDir) -> AdminConfig {
        AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: dir.path().join("tcube.sqlite3"),
            ui_dist: dir.path().join("admin-ui"),
            media_root: dir.path().join("media"),
            content_root: dir.path().join("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
            usb_connected: false,
            version_file: dir.path().join("VERSION"),
            update_dir: dir.path().join("update"),
            update_repo: "campingas/tcube-pi".to_string(),
        }
    }

    #[test]
    fn installed_version_reads_first_nonempty_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("VERSION");
        fs::write(&path, "\n  v0.0.85  \nignored\n").unwrap();
        assert_eq!(read_installed_version(&path).as_deref(), Some("v0.0.85"));
    }

    #[test]
    fn installed_version_missing_file_is_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_installed_version(&dir.path().join("VERSION")), None);
    }

    #[test]
    fn update_available_when_latest_differs() {
        assert!(compute_update_available(Some("v0.0.86"), Some("v0.0.85")));
        assert!(compute_update_available(Some("v0.0.86"), None));
        assert!(!compute_update_available(Some("v0.0.85"), Some("v0.0.85")));
        assert!(!compute_update_available(Some(""), Some("v0.0.85")));
        assert!(!compute_update_available(None, Some("v0.0.85")));
    }

    #[test]
    fn status_prioritizes_running_then_queued() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        fs::create_dir_all(config.update_dir.join("requests")).unwrap();
        fs::write(config.update_dir.join("requests/install"), "install\n").unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 22, 0, 30, 0).unwrap();

        assert_eq!(read_update_status(&config, now).state, STATE_QUEUED);
        fs::write(
            config.update_dir.join("state"),
            "state=running\nstarted_at=2026-07-22T00:00:00Z\n",
        )
        .unwrap();
        assert_eq!(read_update_status(&config, now).state, STATE_RUNNING);
    }

    #[test]
    fn stale_running_state_is_reported_as_failed() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        fs::create_dir_all(&config.update_dir).unwrap();
        fs::write(
            config.update_dir.join("state"),
            "state=running\nstarted_at=2026-07-21T23:00:00Z\n",
        )
        .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 22, 0, 0, 0).unwrap();

        let status = read_update_status(&config, now);
        assert_eq!(status.state, STATE_FAILED);
        assert!(status.error.unwrap().contains("35-minute safety window"));
    }

    #[test]
    fn status_defaults_to_idle() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            read_update_status(&config(&dir), Utc::now()).state,
            STATE_IDLE
        );
    }

    #[test]
    fn log_tail_returns_last_lines_only() {
        let dir = TempDir::new().unwrap();
        assert!(read_log_tail(dir.path(), 3).is_empty());
        fs::write(dir.path().join("log"), "a\n\nb\nc\nd\n").unwrap();
        assert_eq!(read_log_tail(dir.path(), 3), vec!["b", "c", "d"]);
    }
}
