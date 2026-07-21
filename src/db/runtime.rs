//! Read-only view of the admin database for the child-facing runtime.
//!
//! The admin service owns the schema and writes button mappings, soundbox
//! selections, and activated content items. The runtime overlays that state
//! onto the shipped content pack so parent configuration takes effect without
//! coupling the two processes: the only contract is the shared SQLite file.
//! Every read is guarded so a missing or half-migrated database degrades to
//! the static content pack instead of failing the device loop.

use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::db::admin::schema::table_exists;
use crate::events::types::{ButtonBehavior, ButtonMapping, ContentPack, ModeContent, Response};
use crate::hardware::soundbox;

const RUNTIME_BUTTON_IDS: std::ops::RangeInclusive<u8> = 1..=5;

#[derive(Clone, Debug, Default)]
pub(crate) struct RuntimeOverlay {
    pub(crate) setup: Option<DeviceSetupOverlay>,
    pub(crate) mappings: Vec<OverlayMapping>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceSetupOverlay {
    pub(crate) setup_complete: bool,
    pub(crate) dashboard_host: String,
    pub(crate) dashboard_ip: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct OverlayMapping {
    pub(crate) button_id: u8,
    pub(crate) mode: String,
    pub(crate) language: Option<String>,
    pub(crate) responses: Vec<Response>,
}

/// Cheap change fingerprint over the runtime-relevant configuration tables.
/// Compared after `PRAGMA data_version` moves to decide whether a rebuild of
/// the content snapshot is needed.
pub(crate) fn config_fingerprint(conn: &Connection) -> Result<String> {
    let mut parts = Vec::new();
    for table in [
        "device_setup",
        "button_mappings",
        "soundbox_selections",
        "content_items",
        "audio_settings",
    ] {
        if !table_exists(conn, table)? {
            parts.push(format!("{table}:-"));
            continue;
        }
        let sql = format!("select count(*) || ':' || coalesce(max(updated_at), '') from {table}");
        let value: String = conn
            .query_row(&sql, [], |row| row.get(0))
            .with_context(|| format!("failed to fingerprint {table}"))?;
        parts.push(format!("{table}:{value}"));
    }
    Ok(parts.join("|"))
}

pub(crate) fn read_overlay(conn: &Connection) -> Result<RuntimeOverlay> {
    Ok(RuntimeOverlay {
        setup: read_device_setup(conn)?,
        mappings: read_mappings(conn)?,
    })
}

fn read_device_setup(conn: &Connection) -> Result<Option<DeviceSetupOverlay>> {
    if !table_exists(conn, "device_setup")? {
        return Ok(None);
    }
    conn.query_row(
        "select setup_complete, dashboard_host, dashboard_ip from device_setup where id = 1",
        [],
        |row| {
            Ok(DeviceSetupOverlay {
                setup_complete: row.get::<_, i64>(0)? != 0,
                dashboard_host: row.get(1)?,
                dashboard_ip: row.get(2)?,
            })
        },
    )
    .optional()
    .context("failed to read device setup state")
}

fn read_mappings(conn: &Connection) -> Result<Vec<OverlayMapping>> {
    if !table_exists(conn, "button_mappings")? {
        return Ok(Vec::new());
    }
    let mut statement = conn
        .prepare("select button_id, mode, language from button_mappings order by button_id")
        .context("failed to read button mappings")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .context("failed to read button mappings")?;

    let mut mappings = Vec::new();
    for row in rows {
        let (button_id, mode, language) = row.context("failed to read button mapping row")?;
        let Ok(button_id) = u8::try_from(button_id) else {
            continue;
        };
        if !RUNTIME_BUTTON_IDS.contains(&button_id) {
            continue;
        }
        let responses = match mode.as_str() {
            "language" => match language.as_deref() {
                Some(language) => content_responses(conn, button_id, "language", Some(language))?,
                None => Vec::new(),
            },
            "animals" | "music" => content_responses(conn, button_id, &mode, None)?,
            "soundbox" => soundbox_responses(conn, button_id)?,
            _ => Vec::new(),
        };
        mappings.push(OverlayMapping {
            button_id,
            mode,
            language,
            responses,
        });
    }
    Ok(mappings)
}

fn content_responses(
    conn: &Connection,
    button_id: u8,
    content_type: &str,
    language: Option<&str>,
) -> Result<Vec<Response>> {
    if !table_exists(conn, "content_items")? {
        return Ok(Vec::new());
    }
    let sql = if language.is_some() {
        "select id, coalesce(text, title, id), audio_path from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'active' and language = ?3 \
         order by order_index, id"
    } else {
        "select id, coalesce(text, title, id), audio_path from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'active' \
         order by order_index, id"
    };
    let mut statement = conn.prepare(sql).context("failed to read content items")?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(Response {
            id: row.get(0)?,
            text: row.get(1)?,
            audio_path: row.get(2)?,
        })
    };
    let rows = if let Some(language) = language {
        statement.query_map(params![button_id, content_type, language], map_row)?
    } else {
        statement.query_map(params![button_id, content_type], map_row)?
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read content item rows")
}

fn soundbox_responses(conn: &Connection, button_id: u8) -> Result<Vec<Response>> {
    let mut selections: HashMap<String, bool> = HashMap::new();
    if table_exists(conn, "soundbox_selections")? {
        let mut statement = conn
            .prepare("select slug, active from soundbox_selections where button_id = ?1")
            .context("failed to read soundbox selections")?;
        let rows = statement
            .query_map([button_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
            })
            .context("failed to read soundbox selections")?;
        for row in rows {
            let (slug, active) = row.context("failed to read soundbox selection row")?;
            selections.insert(slug, active);
        }
    }

    // A slug without a stored row defaults to active, matching the admin
    // service's ensure_default_selections seeding.
    Ok(soundbox::CATALOG
        .iter()
        .filter(|melody| selections.get(melody.slug).copied().unwrap_or(true))
        .map(|melody| Response {
            id: melody.slug.to_string(),
            text: melody.title.to_string(),
            audio_path: Some(format!("{}{}", soundbox::BUILTIN_PREFIX, melody.slug)),
        })
        .collect())
}

/// Overlays the admin database state onto the shipped content pack. Buttons
/// whose database mapping has no playable content keep their shipped mapping;
/// an empty button_mappings table keeps the shipped pack untouched apart from
/// the device setup state. The merged pack is validated so a half-written
/// admin state can never replace a good snapshot.
pub(crate) fn merge_content(base: &ContentPack, overlay: &RuntimeOverlay) -> Result<ContentPack> {
    let mut pack = base.clone();
    if let Some(setup) = &overlay.setup {
        pack.setup_complete = setup.setup_complete;
        pack.dashboard_host = setup.dashboard_host.clone();
        pack.dashboard_ip = setup.dashboard_ip.clone();
    }
    if overlay.mappings.is_empty() {
        pack.validate()?;
        return Ok(pack);
    }

    // Mode names synthesized from the database, keyed by name so a second
    // button producing the same name gets a disambiguated copy while a
    // matching shipped mode is overridden in place.
    let mut synthesized: HashMap<String, u8> = HashMap::new();
    let mut mappings: Vec<ButtonMapping> = Vec::new();

    for overlay_mapping in &overlay.mappings {
        let mapping = match overlay_mapping.mode.as_str() {
            "disabled" => ButtonMapping {
                button_id: overlay_mapping.button_id,
                behavior: ButtonBehavior::Disabled,
                mode: None,
            },
            "setup_help" => ButtonMapping {
                button_id: overlay_mapping.button_id,
                behavior: ButtonBehavior::SetupHelp,
                mode: None,
            },
            db_mode @ ("language" | "animals" | "music" | "soundbox") => {
                if overlay_mapping.responses.is_empty() {
                    fallback_mapping(base, &pack.modes, overlay_mapping.button_id)
                } else {
                    let behavior = match db_mode {
                        "language" => ButtonBehavior::Language,
                        "animals" => ButtonBehavior::Animals,
                        "music" => ButtonBehavior::Music,
                        _ => ButtonBehavior::Soundbox,
                    };
                    let desired = match (&overlay_mapping.language, db_mode) {
                        (Some(language), "language") => format!("language:{language}"),
                        _ => db_mode.to_string(),
                    };
                    let name = if synthesized.contains_key(&desired) {
                        format!("{desired}:button-{}", overlay_mapping.button_id)
                    } else {
                        desired
                    };
                    synthesized.insert(name.clone(), overlay_mapping.button_id);
                    upsert_mode(&mut pack.modes, &name, overlay_mapping.responses.clone());
                    ButtonMapping {
                        button_id: overlay_mapping.button_id,
                        behavior,
                        mode: Some(name),
                    }
                }
            }
            _ => fallback_mapping(base, &pack.modes, overlay_mapping.button_id),
        };
        mappings.push(mapping);
    }

    for button_id in RUNTIME_BUTTON_IDS {
        if !mappings
            .iter()
            .any(|mapping| mapping.button_id == button_id)
        {
            mappings.push(fallback_mapping(base, &pack.modes, button_id));
        }
    }
    mappings.sort_by_key(|mapping| mapping.button_id);
    pack.button_mappings = mappings;
    pack.validate()?;
    Ok(pack)
}

fn upsert_mode(modes: &mut Vec<ModeContent>, name: &str, responses: Vec<Response>) {
    if let Some(existing) = modes.iter_mut().find(|mode| mode.mode == name) {
        existing.responses = responses;
    } else {
        modes.push(ModeContent {
            mode: name.to_string(),
            responses,
        });
    }
}

/// The shipped mapping for a button, downgraded to disabled when it points at
/// a mode the merged pack does not carry, so the merged pack always validates.
fn fallback_mapping(base: &ContentPack, modes: &[ModeContent], button_id: u8) -> ButtonMapping {
    let disabled = ButtonMapping {
        button_id,
        behavior: ButtonBehavior::Disabled,
        mode: None,
    };
    let Ok(mapping) = base.mapping_for(button_id) else {
        return disabled;
    };
    match mapping.behavior {
        ButtonBehavior::Disabled | ButtonBehavior::SetupHelp => mapping,
        _ => {
            let mode_present = mapping
                .mode
                .as_deref()
                .is_some_and(|name| modes.iter().any(|mode| mode.mode == name));
            if mode_present {
                mapping
            } else {
                disabled
            }
        }
    }
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

    fn response(id: &str, audio_path: &str) -> Response {
        Response {
            id: id.to_string(),
            text: id.to_string(),
            audio_path: Some(audio_path.to_string()),
        }
    }

    fn base_pack() -> ContentPack {
        ContentPack {
            setup_complete: false,
            dashboard_host: "tcube.local".to_string(),
            dashboard_ip: None,
            setup_help_text: "Open t cube dot local to set me up.".to_string(),
            button_mappings: vec![
                ButtonMapping {
                    button_id: 1,
                    behavior: ButtonBehavior::Language,
                    mode: Some("language:English".to_string()),
                },
                ButtonMapping {
                    button_id: 2,
                    behavior: ButtonBehavior::Language,
                    mode: Some("language:Vietnamese".to_string()),
                },
                ButtonMapping {
                    button_id: 3,
                    behavior: ButtonBehavior::Animals,
                    mode: Some("animals".to_string()),
                },
                ButtonMapping {
                    button_id: 4,
                    behavior: ButtonBehavior::Soundbox,
                    mode: Some("soundbox".to_string()),
                },
                ButtonMapping {
                    button_id: 5,
                    behavior: ButtonBehavior::Music,
                    mode: Some("music".to_string()),
                },
            ],
            modes: vec![
                ModeContent {
                    mode: "language:English".to_string(),
                    responses: vec![response("base-en", "content/audio/english/base.wav")],
                },
                ModeContent {
                    mode: "language:Vietnamese".to_string(),
                    responses: vec![response("base-vi", "content/audio/vietnamese/base.wav")],
                },
                ModeContent {
                    mode: "animals".to_string(),
                    responses: vec![response("base-animal", "content/audio/animals/base.wav")],
                },
                ModeContent {
                    mode: "music".to_string(),
                    responses: vec![response("base-music", "content/audio/music/base.mp3")],
                },
                ModeContent {
                    mode: "soundbox".to_string(),
                    responses: soundbox::CATALOG
                        .iter()
                        .map(|melody| Response {
                            id: melody.slug.to_string(),
                            text: melody.title.to_string(),
                            audio_path: Some(format!(
                                "{}{}",
                                soundbox::BUILTIN_PREFIX,
                                melody.slug
                            )),
                        })
                        .collect(),
                },
            ],
        }
    }

    fn merged(conn: &Connection) -> ContentPack {
        let overlay = read_overlay(conn).unwrap();
        merge_content(&base_pack(), &overlay).unwrap()
    }

    fn mapping(pack: &ContentPack, button_id: u8) -> &ButtonMapping {
        pack.button_mappings
            .iter()
            .find(|mapping| mapping.button_id == button_id)
            .unwrap()
    }

    fn mode_responses<'a>(pack: &'a ContentPack, name: &str) -> &'a [Response] {
        &pack
            .modes
            .iter()
            .find(|mode| mode.mode == name)
            .unwrap()
            .responses
    }

    #[test]
    fn seeded_database_overlays_default_mappings() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        let pack = merged(&conn);

        let btn1 = mapping(&pack, 1);
        assert_eq!(btn1.behavior, ButtonBehavior::Language);
        assert_eq!(btn1.mode.as_deref(), Some("language:English"));
        assert_eq!(mode_responses(&pack, "language:English").len(), 10);

        assert_eq!(mapping(&pack, 2).behavior, ButtonBehavior::Animals);
        assert_eq!(mapping(&pack, 3).behavior, ButtonBehavior::Music);
        assert_eq!(mapping(&pack, 4).behavior, ButtonBehavior::SetupHelp);
        assert_eq!(mapping(&pack, 5).behavior, ButtonBehavior::SetupHelp);
        assert!(!pack.setup_complete);
    }

    #[test]
    fn language_filter_excludes_other_languages() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values ('vi-1', 'language', 1, 'Vietnamese', 'Xin chao', 'Xin chao', \
                     'data/audio/active/vi.wav', 'recorded', 'active', 0)",
            [],
        )
        .unwrap();

        let pack = merged(&conn);
        let responses = mode_responses(&pack, "language:English");
        assert!(responses.iter().all(|response| response.id != "vi-1"));
    }

    #[test]
    fn soundbox_respects_inactive_selections() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "update button_mappings set mode = 'soundbox', language = null where button_id = 4",
            [],
        )
        .unwrap();
        let muted = soundbox::CATALOG[0].slug;
        conn.execute(
            "insert into soundbox_selections (button_id, slug, active) values (4, ?1, 0)",
            [muted],
        )
        .unwrap();

        let pack = merged(&conn);
        let btn4 = mapping(&pack, 4);
        assert_eq!(btn4.behavior, ButtonBehavior::Soundbox);
        let responses = mode_responses(&pack, btn4.mode.as_deref().unwrap());
        assert_eq!(responses.len(), soundbox::CATALOG.len() - 1);
        assert!(responses.iter().all(|response| response.id != muted));
    }

    #[test]
    fn soundbox_without_selection_rows_uses_full_catalog() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "update button_mappings set mode = 'soundbox', language = null where button_id = 5",
            [],
        )
        .unwrap();

        let pack = merged(&conn);
        let btn5 = mapping(&pack, 5);
        let responses = mode_responses(&pack, btn5.mode.as_deref().unwrap());
        assert_eq!(responses.len(), soundbox::CATALOG.len());
    }

    #[test]
    fn empty_button_mappings_keeps_base_pack() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute("delete from content_items", []).unwrap();
        conn.execute("delete from button_mappings", []).unwrap();

        let pack = merged(&conn);
        assert_eq!(
            pack.button_mappings.len(),
            base_pack().button_mappings.len()
        );
        assert_eq!(
            mapping(&pack, 2).mode.as_deref(),
            Some("language:Vietnamese")
        );
        assert_eq!(mode_responses(&pack, "language:English").len(), 1);
    }

    #[test]
    fn mapping_without_active_items_falls_back_to_base() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "update button_mappings set mode = 'language', language = 'French' where button_id = 2",
            [],
        )
        .unwrap();

        let pack = merged(&conn);
        let btn2 = mapping(&pack, 2);
        assert_eq!(btn2.behavior, ButtonBehavior::Language);
        assert_eq!(btn2.mode.as_deref(), Some("language:Vietnamese"));
    }

    #[test]
    fn setup_complete_overlays_base_pack() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "update device_setup set setup_complete = 1, dashboard_ip = '192.168.1.20' where id = 1",
            [],
        )
        .unwrap();

        let pack = merged(&conn);
        assert!(pack.setup_complete);
        assert_eq!(pack.dashboard_ip.as_deref(), Some("192.168.1.20"));
    }

    #[test]
    fn duplicate_synthesized_mode_names_are_disambiguated() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);
        conn.execute(
            "update button_mappings set mode = 'language', language = 'English', content_type = 'language' \
             where button_id = 2",
            [],
        )
        .unwrap();
        conn.execute(
            "insert into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values ('en-btn2', 'language', 2, 'English', 'Hi again', 'Hi again', \
                     'data/audio/active/en-btn2.wav', 'recorded', 'active', 0)",
            [],
        )
        .unwrap();

        let pack = merged(&conn);
        let btn1 = mapping(&pack, 1);
        let btn2 = mapping(&pack, 2);
        assert_eq!(btn1.mode.as_deref(), Some("language:English"));
        assert_eq!(btn2.mode.as_deref(), Some("language:English:button-2"));
        assert_eq!(mode_responses(&pack, "language:English:button-2").len(), 1);
    }

    #[test]
    fn fingerprint_tracks_configuration_changes() {
        let dir = TempDir::new().unwrap();
        let conn = test_connection(&dir);

        let first = config_fingerprint(&conn).unwrap();
        let second = config_fingerprint(&conn).unwrap();
        assert_eq!(first, second);

        conn.execute(
            "update button_mappings set mode = 'disabled', updated_at = '2099-01-01T00:00:00Z' \
             where button_id = 5",
            [],
        )
        .unwrap();
        let third = config_fingerprint(&conn).unwrap();
        assert_ne!(first, third);
    }

    #[test]
    fn missing_tables_degrade_to_base_pack() {
        let dir = TempDir::new().unwrap();
        let database = dir.path().join("empty.sqlite3");
        let conn = Connection::open(&database).unwrap();

        let overlay = read_overlay(&conn).unwrap();
        assert!(overlay.setup.is_none());
        assert!(overlay.mappings.is_empty());
        let pack = merge_content(&base_pack(), &overlay).unwrap();
        assert_eq!(pack.button_mappings.len(), 5);
        assert!(config_fingerprint(&conn)
            .unwrap()
            .contains("device_setup:-"));
    }
}
