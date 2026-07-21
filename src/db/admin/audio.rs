use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::auth::now;
use super::schema::table_exists;

pub(crate) const DEFAULT_VOLUME_PERCENT: u8 = 50;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AudioSettings {
    pub(crate) volume_percent: u8,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AudioSettingsUpdate {
    pub(crate) volume_percent: i64,
}

pub(crate) fn default_settings() -> AudioSettings {
    AudioSettings {
        volume_percent: DEFAULT_VOLUME_PERCENT,
        updated_at: now(),
    }
}

pub(crate) fn get_settings(conn: &Connection) -> Result<AudioSettings> {
    if !table_exists(conn, "audio_settings")? {
        return Ok(default_settings());
    }

    conn.query_row(
        "select volume_percent, updated_at from audio_settings where id = 1",
        [],
        |row| {
            Ok(AudioSettings {
                volume_percent: row.get(0)?,
                updated_at: row.get(1)?,
            })
        },
    )
    .optional()
    .context("failed to read audio settings")
    .map(|settings| settings.unwrap_or_else(default_settings))
}

pub(crate) fn save_settings(
    conn: &Connection,
    update: AudioSettingsUpdate,
) -> Result<AudioSettings> {
    let volume_percent = validate_volume_percent(update.volume_percent)?;
    let timestamp = now();
    conn.execute(
        "insert into audio_settings (id, volume_percent, updated_at) values (1, ?1, ?2) \
         on conflict(id) do update set volume_percent = excluded.volume_percent, updated_at = excluded.updated_at",
        params![volume_percent, timestamp],
    )
    .context("failed to save audio settings")?;
    get_settings(conn)
}

fn validate_volume_percent(volume_percent: i64) -> Result<u8> {
    u8::try_from(volume_percent)
        .ok()
        .filter(|value| *value <= 100)
        .context("volume percent must be between 0 and 100")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_table_uses_safe_default() {
        let conn = Connection::open_in_memory().unwrap();

        let settings = get_settings(&conn).unwrap();

        assert_eq!(settings.volume_percent, DEFAULT_VOLUME_PERCENT);
    }

    #[test]
    fn validates_volume_range() {
        assert_eq!(validate_volume_percent(0).unwrap(), 0);
        assert_eq!(validate_volume_percent(100).unwrap(), 100);
        assert!(validate_volume_percent(-1).is_err());
        assert!(validate_volume_percent(101).is_err());
    }
}
