use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::auth::now;
use super::schema::table_exists;

pub(crate) const MIN_FOCUS_MINUTES: u16 = 5;
pub(crate) const MAX_FOCUS_MINUTES: u16 = 60;
pub(crate) const MIN_BREAK_MINUTES: u16 = 1;
pub(crate) const MAX_BREAK_MINUTES: u16 = 30;
pub(crate) const MIN_CYCLES: u8 = 1;
pub(crate) const MAX_CYCLES: u8 = 8;
pub(crate) const MIN_CHILD_AGE_YEARS: u8 = 3;
pub(crate) const MAX_CHILD_AGE_YEARS: u8 = 18;
pub(crate) const TRIGGER_MODE: &str = "any";
pub(crate) const TRIGGER_REQUIRED_BUTTON_COUNT: u8 = 2;
pub(crate) const TRIGGER_ASSEMBLY_WINDOW_MS: u64 = 500;
pub(crate) const TRIGGER_HOLD_SECONDS: u64 = 3;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PomodoroSettings {
    pub(crate) enabled: bool,
    pub(crate) child_age_years: Option<u8>,
    pub(crate) focus_minutes: u16,
    pub(crate) break_minutes: u16,
    pub(crate) cycles: u8,
    pub(crate) preset: String,
    pub(crate) validated_at: Option<String>,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PomodoroRecommendation {
    pub(crate) preset: String,
    pub(crate) focus_minutes: u16,
    pub(crate) break_minutes: u16,
    pub(crate) cycles: u8,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PomodoroSettingsWithRecommendation {
    #[serde(flatten)]
    pub(crate) settings: PomodoroSettings,
    pub(crate) recommendation: PomodoroRecommendation,
    pub(crate) trigger: PomodoroTriggerMetadata,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PomodoroTriggerMetadata {
    pub(crate) mode: String,
    pub(crate) required_button_count: u8,
    pub(crate) assembly_window_ms: u64,
    pub(crate) hold_seconds: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PomodoroSettingsUpdate {
    pub(crate) enabled: bool,
    pub(crate) child_age_years: Option<u8>,
    pub(crate) focus_minutes: u16,
    pub(crate) break_minutes: u16,
    pub(crate) cycles: u8,
    pub(crate) preset: String,
}

pub(crate) fn default_settings() -> PomodoroSettings {
    PomodoroSettings {
        enabled: false,
        child_age_years: None,
        focus_minutes: 10,
        break_minutes: 3,
        cycles: 2,
        preset: "mini".to_string(),
        validated_at: None,
        updated_at: now(),
    }
}

pub(crate) fn recommendation_for_age(age: Option<u8>) -> PomodoroRecommendation {
    match age {
        Some(3..=5) => PomodoroRecommendation {
            preset: "mini".to_string(),
            focus_minutes: 8,
            break_minutes: 3,
            cycles: 2,
            reason: "Starter plan for ages 3-5.".to_string(),
        },
        Some(6..=8) => PomodoroRecommendation {
            preset: "mini".to_string(),
            focus_minutes: 12,
            break_minutes: 4,
            cycles: 3,
            reason: "Starter plan for ages 6-8.".to_string(),
        },
        Some(9..=12) => PomodoroRecommendation {
            preset: "focus".to_string(),
            focus_minutes: 20,
            break_minutes: 5,
            cycles: 3,
            reason: "Focus plan for ages 9-12.".to_string(),
        },
        Some(_) => PomodoroRecommendation {
            preset: "full".to_string(),
            focus_minutes: 25,
            break_minutes: 5,
            cycles: 4,
            reason: "Full plan for ages 13 and up.".to_string(),
        },
        None => PomodoroRecommendation {
            preset: "mini".to_string(),
            focus_minutes: 10,
            break_minutes: 3,
            cycles: 2,
            reason: "Starter plan until an owner saves the child age.".to_string(),
        },
    }
}

pub(crate) fn get_settings(conn: &Connection) -> Result<PomodoroSettingsWithRecommendation> {
    if !table_exists(conn, "pomodoro_settings")? {
        let settings = default_settings();
        return Ok(with_recommendation(settings));
    }

    let settings = conn
        .prepare(
            "select enabled, child_age_years, focus_minutes, break_minutes, cycles, preset, validated_at, updated_at \
             from pomodoro_settings where id = 1",
        )?
        .query_row([], |row| {
            Ok(PomodoroSettings {
                enabled: row.get::<_, i64>(0)? != 0,
                child_age_years: row.get::<_, Option<u8>>(1)?,
                focus_minutes: row.get::<_, u16>(2)?,
                break_minutes: row.get::<_, u16>(3)?,
                cycles: row.get::<_, u8>(4)?,
                preset: row.get(5)?,
                validated_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .optional()
        .context("failed to read pomodoro settings")?
        .unwrap_or_else(default_settings);
    Ok(with_recommendation(settings))
}

pub(crate) fn save_settings(
    conn: &Connection,
    update: PomodoroSettingsUpdate,
) -> Result<PomodoroSettingsWithRecommendation> {
    validate_update(&update)?;
    let timestamp = now();
    let validated_at = timestamp.clone();
    conn.execute(
        "insert into pomodoro_settings \
         (id, enabled, child_age_years, focus_minutes, break_minutes, cycles, preset, validated_at, updated_at) \
         values (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         on conflict(id) do update set \
         enabled = excluded.enabled, \
         child_age_years = excluded.child_age_years, \
         focus_minutes = excluded.focus_minutes, \
         break_minutes = excluded.break_minutes, \
         cycles = excluded.cycles, \
         preset = excluded.preset, \
         validated_at = excluded.validated_at, \
         updated_at = excluded.updated_at",
        params![
            update.enabled,
            update.child_age_years,
            update.focus_minutes,
            update.break_minutes,
            update.cycles,
            update.preset,
            validated_at,
            timestamp
        ],
    )
    .context("failed to save pomodoro settings")?;
    get_settings(conn)
}

pub(crate) fn validate_update(update: &PomodoroSettingsUpdate) -> Result<()> {
    if let Some(age) = update.child_age_years {
        if !(MIN_CHILD_AGE_YEARS..=MAX_CHILD_AGE_YEARS).contains(&age) {
            anyhow::bail!(
                "child age must be between {MIN_CHILD_AGE_YEARS} and {MAX_CHILD_AGE_YEARS}"
            );
        }
    }
    if !(MIN_FOCUS_MINUTES..=MAX_FOCUS_MINUTES).contains(&update.focus_minutes) {
        anyhow::bail!("focus minutes must be between {MIN_FOCUS_MINUTES} and {MAX_FOCUS_MINUTES}");
    }
    if !(MIN_BREAK_MINUTES..=MAX_BREAK_MINUTES).contains(&update.break_minutes) {
        anyhow::bail!("break minutes must be between {MIN_BREAK_MINUTES} and {MAX_BREAK_MINUTES}");
    }
    if !(MIN_CYCLES..=MAX_CYCLES).contains(&update.cycles) {
        anyhow::bail!("cycles must be between {MIN_CYCLES} and {MAX_CYCLES}");
    }
    if !matches!(update.preset.as_str(), "mini" | "focus" | "full" | "custom") {
        anyhow::bail!("preset must be mini, focus, full, or custom");
    }
    if update.enabled && update.child_age_years.is_none() {
        anyhow::bail!("child age is required before enabling the focus routine");
    }
    Ok(())
}

pub(crate) fn runtime_enabled_settings(conn: &Connection) -> Result<Option<PomodoroSettings>> {
    let settings = get_settings(conn)?.settings;
    if settings.enabled && settings.validated_at.is_some() {
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

fn with_recommendation(settings: PomodoroSettings) -> PomodoroSettingsWithRecommendation {
    PomodoroSettingsWithRecommendation {
        recommendation: recommendation_for_age(settings.child_age_years),
        trigger: PomodoroTriggerMetadata {
            mode: TRIGGER_MODE.to_string(),
            required_button_count: TRIGGER_REQUIRED_BUTTON_COUNT,
            assembly_window_ms: TRIGGER_ASSEMBLY_WINDOW_MS,
            hold_seconds: TRIGGER_HOLD_SECONDS,
        },
        settings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_age_to_recommended_plan() {
        assert_eq!(
            recommendation_for_age(Some(4)),
            PomodoroRecommendation {
                preset: "mini".to_string(),
                focus_minutes: 8,
                break_minutes: 3,
                cycles: 2,
                reason: "Starter plan for ages 3-5.".to_string(),
            }
        );
        assert_eq!(recommendation_for_age(Some(7)).focus_minutes, 12);
        assert_eq!(recommendation_for_age(Some(10)).preset, "focus");
        assert_eq!(recommendation_for_age(Some(13)).cycles, 4);
        assert_eq!(recommendation_for_age(None).focus_minutes, 10);
    }

    #[test]
    fn validates_bounds_and_enabled_age_requirement() {
        let valid = PomodoroSettingsUpdate {
            enabled: true,
            child_age_years: Some(9),
            focus_minutes: 20,
            break_minutes: 5,
            cycles: 3,
            preset: "focus".to_string(),
        };
        assert!(validate_update(&valid).is_ok());
        assert!(validate_update(&PomodoroSettingsUpdate {
            focus_minutes: 4,
            ..valid.clone()
        })
        .is_err());
        assert!(validate_update(&PomodoroSettingsUpdate {
            break_minutes: 31,
            ..valid.clone()
        })
        .is_err());
        assert!(validate_update(&PomodoroSettingsUpdate {
            cycles: 0,
            ..valid.clone()
        })
        .is_err());
        assert!(validate_update(&PomodoroSettingsUpdate {
            child_age_years: None,
            ..valid.clone()
        })
        .is_err());
        assert!(validate_update(&PomodoroSettingsUpdate {
            preset: "unsupported".to_string(),
            ..valid
        })
        .is_err());
    }
}
