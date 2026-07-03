use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

use crate::hardware::soundbox::{melody_for_slug, CATALOG};

pub(crate) fn ensure_default_selections(conn: &Connection, button_id: u8) -> Result<()> {
    for melody in &CATALOG {
        conn.execute(
            "insert or ignore into soundbox_selections (button_id, slug, active) \
             values (?1, ?2, 1)",
            params![button_id, melody.slug],
        )
        .context("failed to seed soundbox selections")?;
    }
    Ok(())
}

pub(crate) fn selections_for_button(
    conn: &Connection,
    button_id: u8,
) -> Result<HashMap<String, bool>> {
    let mut statement = conn
        .prepare("select slug, active from soundbox_selections where button_id = ?1")
        .context("failed to read soundbox selections")?;
    let rows = statement
        .query_map([button_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })
        .context("failed to read soundbox selections")?;
    let mut selections = HashMap::new();
    for row in rows {
        let (slug, active) = row.context("failed to read soundbox selection row")?;
        selections.insert(slug, active);
    }
    Ok(selections)
}

pub(crate) fn set_selection(
    conn: &Connection,
    button_id: u8,
    slug: &str,
    active: bool,
) -> Result<()> {
    if melody_for_slug(slug).is_none() {
        bail!("unknown SoundBox sound {slug}");
    }
    ensure_default_selections(conn, button_id)?;
    if !active {
        let remaining: i64 = conn
            .query_row(
                "select count(*) from soundbox_selections \
                 where button_id = ?1 and active = 1 and slug != ?2",
                params![button_id, slug],
                |row| row.get(0),
            )
            .context("failed to count active soundbox selections")?;
        if remaining == 0 {
            bail!("at least one SoundBox sound must stay active");
        }
    }
    conn.execute(
        "update soundbox_selections \
         set active = ?3, updated_at = current_timestamp \
         where button_id = ?1 and slug = ?2",
        params![button_id, slug, active],
    )
    .context("failed to update soundbox selection")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdminConfig;
    use crate::db::admin::schema::migrate_admin_database;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_connection(dir: &TempDir) -> Connection {
        let database = dir.path().join("tcube.sqlite3");
        let config = AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: database.clone(),
            ui_dist: PathBuf::from("admin-ui"),
            media_root: dir.path().join("media"),
            content_root: dir.path().join("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
            usb_connected: false,
        };
        let conn = Connection::open(&database).unwrap();
        migrate_admin_database(&conn, &config).unwrap();
        conn
    }

    #[test]
    fn seeds_all_catalog_sounds_active_by_default() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);

        ensure_default_selections(&conn, 4).unwrap();
        let selections = selections_for_button(&conn, 4).unwrap();

        assert_eq!(selections.len(), CATALOG.len());
        assert!(selections.values().all(|active| *active));
    }

    #[test]
    fn toggles_selection_round_trip() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        ensure_default_selections(&conn, 2).unwrap();

        set_selection(&conn, 2, "korobeiniki", false).unwrap();
        assert!(!selections_for_button(&conn, 2).unwrap()["korobeiniki"]);

        set_selection(&conn, 2, "korobeiniki", true).unwrap();
        assert!(selections_for_button(&conn, 2).unwrap()["korobeiniki"]);
    }

    #[test]
    fn rejects_deactivating_last_active_sound() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        ensure_default_selections(&conn, 1).unwrap();

        let mut slugs: Vec<&str> = CATALOG.iter().map(|melody| melody.slug).collect();
        let last = slugs.pop().unwrap();
        for slug in slugs {
            set_selection(&conn, 1, slug, false).unwrap();
        }

        assert!(set_selection(&conn, 1, last, false).is_err());
        assert!(selections_for_button(&conn, 1).unwrap()[last]);
    }

    #[test]
    fn rejects_unknown_slug() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);

        assert!(set_selection(&conn, 1, "not-a-melody", true).is_err());
    }

    #[test]
    fn selections_are_scoped_per_button() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        ensure_default_selections(&conn, 1).unwrap();
        ensure_default_selections(&conn, 2).unwrap();

        set_selection(&conn, 1, "twinkle-twinkle", false).unwrap();

        assert!(!selections_for_button(&conn, 1).unwrap()["twinkle-twinkle"]);
        assert!(selections_for_button(&conn, 2).unwrap()["twinkle-twinkle"]);
    }
}
