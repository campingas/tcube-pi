use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::events::types::Measurement;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "create table if not exists button_events (
            id integer primary key autoincrement,
            occurred_at text not null,
            button_id integer not null,
            mode text not null,
            response_id text not null,
            response_text text not null
        );
        create table if not exists setup_debug_events (
            id integer primary key autoincrement,
            occurred_at text not null default current_timestamp,
            event_type text not null,
            button_id integer,
            details text
        );
        create table if not exists latency_measurements (
            id integer primary key autoincrement,
            occurred_at integer not null,
            button_id integer not null,
            mode text not null,
            response_id text not null,
            response_text text not null,
            latency_us integer not null
        );",
    )
    .context("failed to initialize measurement SQLite schema")
}

pub fn insert_measurement(conn: &Connection, m: &Measurement) -> Result<()> {
    conn.execute(
        "insert into latency_measurements \
         (occurred_at, button_id, mode, response_id, response_text, latency_us) \
         values (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            m.occurred_at,
            m.button_id,
            m.mode,
            m.response_id,
            m.response_text,
            m.latency_us as i64
        ],
    )?;
    Ok(())
}

pub fn query_latency_range(conn: &Connection, from: i64, to: i64) -> Result<Vec<Measurement>> {
    let mut stmt = conn.prepare(
        "select occurred_at, button_id, mode, response_id, response_text, latency_us \
         from latency_measurements where occurred_at >= ?1 and occurred_at <= ?2 \
         order by occurred_at asc, id asc",
    )?;
    let rows = stmt.query_map(params![from, to], |row| {
        Ok(Measurement {
            occurred_at: row.get(0)?,
            button_id: row.get(1)?,
            mode: row.get(2)?,
            response_id: row.get(3)?,
            response_text: row.get(4)?,
            latency_us: row.get::<_, i64>(5)? as u128,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
