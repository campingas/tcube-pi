use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::config::AdminConfig;

use super::auth::{generate_uuid_v4, now};
use super::schema::{migrate_admin_database, seed_admin_defaults, table_count, table_exists};

const FACTORY_RESET_TABLES: &[&str] = &[
    "content_package_failures",
    "content_packages",
    "cube_invitations",
    "recovery_codes",
    "admin_sessions",
    "cube_memberships",
    "admin_accounts",
    "media_artifacts",
    "content_jobs",
    "content_items",
    "button_events",
    "setup_debug_events",
    "trusted_sessions",
    "button_mappings",
    "device_setup",
    "devices",
];

#[derive(Debug)]
pub(crate) struct SetupReview {
    pub(crate) cube_name: String,
    pub(crate) device_id: Option<String>,
    pub(crate) admin_created: bool,
    pub(crate) wifi_verified: bool,
    pub(crate) dashboard_ip: Option<String>,
    pub(crate) dashboard_address: String,
    pub(crate) button_modes: HashMap<String, String>,
    pub(crate) active_counts: HashMap<String, i64>,
}

#[derive(Debug)]
pub(crate) struct CubeSave {
    pub(crate) device_id: String,
    pub(crate) name: String,
}

pub(crate) fn setup_review_from_conn(
    config: &AdminConfig,
    conn: &Connection,
) -> Result<SetupReview> {
    if !table_exists(conn, "device_setup")? {
        return Ok(default_setup_review(config));
    }

    let setup = conn
        .prepare(
            "select cube_name, device_id, wifi_verified_at, dashboard_host, dashboard_ip \
             from device_setup where id = 1",
        )?
        .query_row([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        });
    let (cube_name, device_id, wifi_verified_at, dashboard_host, dashboard_ip) = setup
        .unwrap_or_else(|_| {
            (
                Some("T-Cube".to_string()),
                None,
                None,
                config.hostname.clone(),
                None,
            )
        });

    Ok(SetupReview {
        cube_name: cube_name.unwrap_or_else(|| "T-Cube".to_string()),
        device_id,
        admin_created: table_count(conn, "admin_accounts")? > 0,
        wifi_verified: wifi_verified_at.is_some(),
        dashboard_ip,
        dashboard_address: format!("https://{dashboard_host}/"),
        button_modes: button_modes(conn)?,
        active_counts: active_counts(conn)?,
    })
}

pub(crate) fn default_setup_review(config: &AdminConfig) -> SetupReview {
    let mut button_modes = HashMap::new();
    button_modes.insert("1".to_string(), "language:English".to_string());
    button_modes.insert("2".to_string(), "animals".to_string());
    button_modes.insert("3".to_string(), "music".to_string());
    button_modes.insert("4".to_string(), "setup_help".to_string());
    button_modes.insert("5".to_string(), "setup_help".to_string());

    SetupReview {
        cube_name: "T-Cube".to_string(),
        device_id: None,
        admin_created: false,
        wifi_verified: false,
        dashboard_ip: None,
        dashboard_address: format!("https://{}/", config.hostname),
        button_modes,
        active_counts: HashMap::new(),
    }
}

pub(crate) fn save_cube_name(
    conn: &Connection,
    config: &AdminConfig,
    account_id: &str,
    name: &str,
) -> Result<CubeSave> {
    ensure_setup_row(conn, config)?;
    let device_id = conn
        .prepare("select device_id from device_setup where id = 1")?
        .query_row([], |row| row.get::<_, Option<String>>(0))
        .optional()?
        .flatten()
        .unwrap_or_else(generate_uuid_v4);
    conn.execute(
        "insert into devices (id, label, token_hash, created_at, revoked_at) \
         values (?1, ?2, ?3, ?4, null) \
         on conflict(id) do update set label = excluded.label",
        params![device_id, name, "0".repeat(64), now()],
    )?;
    conn.execute(
        "update device_setup set cube_name = ?1, device_id = ?2, updated_at = ?3 where id = 1",
        params![name, device_id, now()],
    )?;
    conn.execute(
        "insert or ignore into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, 'owner', ?3)",
        params![account_id, device_id, now()],
    )?;
    Ok(CubeSave {
        device_id,
        name: name.to_string(),
    })
}

pub(crate) fn verify_wifi(
    conn: &Connection,
    config: &AdminConfig,
    ssid: &str,
    dashboard_ip: &str,
) -> Result<()> {
    ensure_setup_row(conn, config)?;
    conn.execute(
        "update device_setup \
         set wifi_ssid = ?1, wifi_verified_at = ?2, dashboard_ip = ?3, updated_at = ?4 \
         where id = 1",
        params![ssid, now(), dashboard_ip, now()],
    )?;
    Ok(())
}

pub(crate) fn set_button_mode(
    conn: &Connection,
    button_id: i64,
    mode: &str,
    selected_language: Option<&str>,
) -> Result<()> {
    let content_type = match mode {
        "language" | "animals" | "music" => Some(mode),
        _ => None,
    };
    conn.execute(
        "insert into button_mappings \
         (button_id, mode, language, content_type, manual_order_weight, updated_at) \
         values (?1, ?2, ?3, ?4, ?5, ?6) \
         on conflict(button_id) do update set \
         mode = excluded.mode, language = excluded.language, content_type = excluded.content_type, updated_at = excluded.updated_at",
        params![button_id, mode, selected_language, content_type, button_id - 1, now()],
    )?;
    Ok(())
}

pub(crate) fn mark_setup_complete(conn: &Connection) -> Result<()> {
    conn.execute(
        "update device_setup set setup_complete = 1, updated_at = ?1 where id = 1",
        [now()],
    )?;
    Ok(())
}

pub(crate) fn factory_reset_database(conn: &mut Connection, config: &AdminConfig) -> Result<()> {
    migrate_admin_database(conn, config)?;
    let tx = conn
        .transaction()
        .context("failed to start factory reset transaction")?;
    for table in FACTORY_RESET_TABLES {
        tx.execute(&format!("delete from {table}"), [])
            .with_context(|| format!("failed to clear {table} during factory reset"))?;
    }
    tx.execute(
        "delete from sqlite_sequence where name in ('setup_debug_events', 'button_events', 'content_package_failures')",
        [],
    )
    .context("failed to reset factory reset sequences")?;
    seed_admin_defaults(&tx, config)?;
    tx.commit()
        .context("failed to commit factory reset transaction")?;
    Ok(())
}

fn ensure_setup_row(conn: &Connection, config: &AdminConfig) -> Result<()> {
    conn.execute(
        "insert or ignore into device_setup (id, dashboard_host) values (1, ?1)",
        [config.hostname.as_str()],
    )?;
    Ok(())
}

fn button_modes(conn: &Connection) -> Result<HashMap<String, String>> {
    if !table_exists(conn, "button_mappings")? {
        return Ok(HashMap::new());
    }

    let mut stmt =
        conn.prepare("select button_id, mode, language from button_mappings order by button_id")?;
    let rows = stmt.query_map([], |row| {
        let button_id: i64 = row.get(0)?;
        let mode: String = row.get(1)?;
        let language: Option<String> = row.get(2)?;
        let label = if mode == "language" {
            format!(
                "language:{}",
                language.unwrap_or_else(|| "English".to_string())
            )
        } else {
            mode
        };
        Ok((button_id.to_string(), label))
    })?;

    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .context("failed to read button mappings")
}

fn active_counts(conn: &Connection) -> Result<HashMap<String, i64>> {
    if !table_exists(conn, "content_items")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn.prepare(
        "select content_type, count(*) from content_items where state = 'active' group by content_type",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .context("failed to read active content counts")
}
