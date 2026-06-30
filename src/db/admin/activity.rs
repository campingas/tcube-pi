use anyhow::Result;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub(crate) struct NewActivityEvent<'a> {
    pub(crate) kind: &'a str,
    pub(crate) occurred_at: &'a str,
    pub(crate) account_id: Option<&'a str>,
    pub(crate) button_id: Option<i64>,
    pub(crate) content_id: Option<&'a str>,
    pub(crate) content_type: Option<&'a str>,
    pub(crate) content_title: Option<&'a str>,
    pub(crate) audio_path: Option<&'a str>,
    pub(crate) source: Option<&'a str>,
    pub(crate) detail: Option<&'a str>,
}

pub(crate) fn record_activity_event(conn: &Connection, event: &NewActivityEvent<'_>) -> Result<()> {
    conn.execute(
        "insert into admin_activity_events \
         (occurred_at, kind, account_id, button_id, content_id, content_type, content_title, audio_path, source, detail) \
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            event.occurred_at,
            event.kind,
            event.account_id,
            event.button_id,
            event.content_id,
            event.content_type,
            event.content_title,
            event.audio_path,
            event.source,
            event.detail,
        ],
    )?;
    Ok(())
}
