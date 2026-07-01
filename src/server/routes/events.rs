use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::config::AdminConfig;
use crate::db::admin::auth::{authenticate_session, require_local_cube_role, RoleRequirement};
use crate::db::admin::schema::table_exists;

const RECENT_ACTIVITY_LIMIT: usize = 10;

#[derive(Debug, Serialize)]
pub(crate) struct RecentActivityEventResponse {
    id: String,
    kind: String,
    occurred_at: String,
    button_id: Option<i64>,
    button_label: Option<String>,
    mode: Option<String>,
    response_id: Option<String>,
    response_text: Option<String>,
    content_id: Option<String>,
    content_type: Option<String>,
    content_title: Option<String>,
    audio_filename: Option<String>,
    source: Option<String>,
    text: Option<String>,
}

pub(crate) fn recent_button_events(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<Vec<RecentActivityEventResponse>> {
    let conn = authenticated_connection(config, token)?;
    let mut events = Vec::new();
    if table_exists(&conn, "button_events")? {
        let mut stmt = conn.prepare(
            "select id, occurred_at, button_id, mode, response_id, response_text \
             from button_events order by occurred_at desc, id desc limit 50",
        )?;
        let rows = stmt.query_map([], |row| {
            let id = row.get::<_, i64>(0)?;
            let button_id = row.get::<_, i64>(2)?;
            let response_id = row.get::<_, String>(4)?;
            let response_text = row.get::<_, String>(5)?;
            Ok(RecentActivityEventResponse {
                id: format!("button:{id}"),
                kind: "button_pressed".to_string(),
                occurred_at: row.get(1)?,
                button_id: Some(button_id),
                button_label: Some(button_label(button_id).to_string()),
                mode: Some(row.get(3)?),
                response_id: Some(response_id),
                response_text: Some(response_text.clone()),
                content_id: None,
                content_type: None,
                content_title: None,
                audio_filename: None,
                source: None,
                text: Some(response_text),
            })
        })?;
        events.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    if table_exists(&conn, "admin_activity_events")? {
        let mut stmt = conn.prepare(
            "select id, occurred_at, kind, button_id, content_id, content_type, content_title, audio_path, source, detail \
             from admin_activity_events order by occurred_at desc, id desc limit 50",
        )?;
        let rows = stmt.query_map([], |row| {
            let id = row.get::<_, i64>(0)?;
            let button_id = row.get::<_, Option<i64>>(3)?;
            let audio_path = row.get::<_, Option<String>>(7)?;
            Ok(RecentActivityEventResponse {
                id: format!("activity:{id}"),
                kind: row.get(2)?,
                occurred_at: row.get(1)?,
                button_id,
                button_label: button_id.map(|id| button_label(id).to_string()),
                mode: None,
                response_id: None,
                response_text: None,
                content_id: row.get(4)?,
                content_type: row.get(5)?,
                content_title: row.get(6)?,
                audio_filename: audio_path.as_deref().and_then(audio_filename),
                source: row.get(8)?,
                text: row.get(9)?,
            })
        })?;
        events.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }

    events.sort_by(|left, right| {
        right
            .occurred_at
            .cmp(&left.occurred_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    events.truncate(RECENT_ACTIVITY_LIMIT);
    Ok(events)
}

fn authenticated_connection(config: &AdminConfig, token: Option<&str>) -> Result<Connection> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    require_local_cube_role(&conn, &session.account.id, RoleRequirement::Member)?;
    Ok(conn)
}

fn button_label(button_id: i64) -> &'static str {
    match button_id {
        1 => "Top",
        2 => "Front left",
        3 => "Front",
        4 => "Front right",
        5 => "Back",
        _ => "Button",
    }
}

fn audio_filename(audio_path: &str) -> Option<String> {
    Path::new(audio_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}
