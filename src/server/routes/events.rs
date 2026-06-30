use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::config::AdminConfig;
use crate::db::admin::auth::{authenticate_session, require_local_cube_role, RoleRequirement};
use crate::db::admin::schema::table_exists;

#[derive(Debug, Serialize)]
pub(crate) struct RecentButtonEventResponse {
    occurred_at: String,
    button_id: i64,
    mode: String,
    response_id: String,
    response_text: String,
}

pub(crate) fn recent_button_events(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<Vec<RecentButtonEventResponse>> {
    let conn = authenticated_connection(config, token)?;
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
