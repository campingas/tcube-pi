use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::auth::{now, random_token};
use super::schema::table_exists;

#[derive(Debug)]
pub(crate) struct ContentItemRow {
    pub(crate) id: String,
    pub(crate) content_type: String,
    pub(crate) title: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) source: String,
    pub(crate) state: String,
    pub(crate) audio_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ContentInventoryRow {
    pub(crate) id: String,
    pub(crate) content_type: String,
    pub(crate) title: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) source: String,
    pub(crate) state: String,
    pub(crate) audio_path: Option<String>,
    pub(crate) button_id: i64,
}

#[derive(Debug)]
pub(crate) struct ContentEmptyState {
    pub(crate) title: String,
    pub(crate) detail: String,
}

#[derive(Debug)]
pub(crate) struct NewContentItem<'a> {
    pub(crate) id: &'a str,
    pub(crate) content_type: &'a str,
    pub(crate) button_id: i64,
    pub(crate) language: Option<&'a str>,
    pub(crate) title: &'a str,
    pub(crate) text: &'a str,
    pub(crate) audio_path: &'a str,
    pub(crate) source: &'a str,
    pub(crate) order_index: i64,
}

#[derive(Clone, Debug)]
struct CurrentButtonMapping {
    mode: String,
    language: Option<String>,
}

pub(crate) fn insert_content_item(conn: &Connection, item: &NewContentItem<'_>) -> Result<()> {
    conn.execute(
        "insert into content_items \
         (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'archived', ?9)",
        params![
            item.id,
            item.content_type,
            item.button_id,
            item.language,
            item.title,
            item.text,
            item.audio_path,
            item.source,
            item.order_index
        ],
    )?;
    Ok(())
}

pub(crate) fn activate_content_item(
    conn: &Connection,
    item_id: &str,
    audio_path: &str,
) -> Result<()> {
    conn.execute(
        "update content_items set state = 'active', audio_path = ?1, updated_at = ?2 where id = ?3",
        params![audio_path, now(), item_id],
    )?;
    Ok(())
}

pub(crate) fn trash_content_item(
    conn: &Connection,
    item_id: &str,
    trashed_at: &str,
    purge_after: &str,
) -> Result<usize> {
    conn.execute(
        "update content_items \
         set state = 'trash', trashed_at = ?1, purge_after = ?2, updated_at = ?3 \
         where id = ?4",
        params![trashed_at, purge_after, trashed_at, item_id],
    )
    .context("failed to trash content item")
}

pub(crate) fn trash_content_items(
    conn: &Connection,
    items: &[ContentInventoryRow],
    trashed_at: &str,
    purge_after: &str,
) -> Result<()> {
    for item in items {
        trash_content_item(conn, &item.id, trashed_at, purge_after)?;
    }
    Ok(())
}

pub(crate) fn trash_generated_language_drafts(
    conn: &Connection,
    button_id: i64,
    language: &str,
    trashed_at: &str,
    purge_after: &str,
) -> Result<usize> {
    conn.execute(
        "update content_items \
         set state = 'trash', trashed_at = ?1, purge_after = ?2, updated_at = ?3 \
         where source in ('recorded', 'uploaded', 'generated') and state = 'archived' and content_type = 'language' \
           and button_id = ?4 and language = ?5",
        params![trashed_at, purge_after, trashed_at, button_id, language],
    )
    .context("failed to trash generated speech drafts")
}

pub(crate) fn active_content_rows(
    conn: &Connection,
    button_id: i64,
    content_type: &str,
    language: Option<&str>,
) -> Result<Vec<ContentItemRow>> {
    let language = if content_type == "language" {
        language.map(str::trim).filter(|value| !value.is_empty())
    } else {
        None
    };
    let sql = if language.is_some() {
        "select id, content_type, title, text, language, source, state, audio_path \
         from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'active' and language = ?3 \
         order by order_index, id"
    } else {
        "select id, content_type, title, text, language, source, state, audio_path \
         from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'active' \
         order by order_index, id"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(language) = language {
        stmt.query_map(
            params![button_id, content_type, language],
            content_item_from_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        stmt.query_map(params![button_id, content_type], content_item_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    Ok(rows)
}

pub(crate) fn inactive_content_rows(
    conn: &Connection,
    button_id: i64,
    content_type: &str,
    language: Option<&str>,
) -> Result<Vec<ContentItemRow>> {
    let sql = if language.is_some() {
        "select id, content_type, title, text, language, source, state, audio_path \
         from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'archived' \
           and language = ?3 and source in ('recorded', 'uploaded', 'generated') \
         order by created_at desc, id"
    } else {
        "select id, content_type, title, text, language, source, state, audio_path \
         from content_items \
         where button_id = ?1 and content_type = ?2 and state = 'archived' \
           and source in ('recorded', 'uploaded', 'generated') \
         order by created_at desc, id"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(language) = language {
        stmt.query_map(
            params![button_id, content_type, language],
            content_item_from_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        stmt.query_map(params![button_id, content_type], content_item_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    Ok(rows)
}

pub(crate) fn content_empty_state(
    conn: &Connection,
    button_id: i64,
    content_type: &str,
    language: Option<&str>,
    state: &str,
) -> Result<Option<ContentEmptyState>> {
    if !table_exists(conn, "content_items")? {
        return Ok(Some(empty_state_no_content(content_type, language, state)));
    }

    if content_type == "language" {
        let same_button_other_languages =
            content_languages_for_button(conn, button_id, state, language)?;
        if !same_button_other_languages.is_empty() {
            return Ok(Some(ContentEmptyState {
                title: empty_title(content_type, language, state),
                detail: format!(
                    "This button has {} content in {}. Switch language back or add new content for {}.",
                    state_label(state),
                    join_labels(&same_button_other_languages),
                    language.unwrap_or("this language")
                ),
            }));
        }

        if let Some(language) = language {
            let other_buttons = content_buttons_for_language(conn, button_id, language, state)?;
            if !other_buttons.is_empty() {
                return Ok(Some(ContentEmptyState {
                    title: empty_title(content_type, Some(language), state),
                    detail: format!(
                        "{language} {} content exists on {}. This button starts with its own empty content set.",
                        state_label(state),
                        join_button_labels(&other_buttons)
                    ),
                }));
            }
        }
    } else {
        let other_buttons = content_buttons_for_type(conn, button_id, content_type, state)?;
        if !other_buttons.is_empty() {
            return Ok(Some(ContentEmptyState {
                title: empty_title(content_type, None, state),
                detail: format!(
                    "{} {} content exists on {}. This button starts with its own empty content set.",
                    content_type_label(content_type),
                    state_label(state),
                    join_button_labels(&other_buttons)
                ),
            }));
        }
    }

    let matching_count = count_content_rows(conn, content_type, language, state)?;
    if matching_count > 0 {
        return Ok(Some(ContentEmptyState {
            title: empty_title(content_type, language, state),
            detail: format!(
                "{} content exists elsewhere, but not for this button selection. Add content here to make this button playable.",
                content_type_label(content_type)
            ),
        }));
    }

    Ok(Some(empty_state_no_content(content_type, language, state)))
}

pub(crate) fn content_inventory_rows(conn: &Connection) -> Result<Vec<ContentInventoryRow>> {
    let mut stmt = conn.prepare(
        "select id, content_type, title, text, language, source, state, audio_path, button_id \
         from content_items \
         where state in ('active', 'archived') and audio_path is not null \
         order by button_id, content_type, language, state, order_index, id",
    )?;
    let rows = stmt
        .query_map([], inventory_item_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read content inventory")?;
    Ok(rows)
}

pub(crate) fn inventory_status(
    conn: &Connection,
    item: &ContentInventoryRow,
) -> Result<(&'static str, String)> {
    let mappings = current_button_mappings(conn)?;
    Ok(inventory_status_for_mapping(
        item,
        mappings.get(&item.button_id),
    ))
}

pub(crate) fn unused_content_items(conn: &Connection) -> Result<Vec<ContentInventoryRow>> {
    let mappings = current_button_mappings(conn)?;
    let mut stmt = conn.prepare(
        "select id, content_type, title, text, language, source, state, audio_path, button_id \
         from content_items \
         where source in ('recorded', 'uploaded', 'generated') and state = 'active' and audio_path is not null \
         order by button_id, content_type, language, order_index, id",
    )?;
    let rows = stmt
        .query_map([], inventory_item_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read unused content rows")?;
    Ok(rows
        .into_iter()
        .filter(|item| {
            inventory_status_for_mapping(item, mappings.get(&item.button_id)).0 == "unused"
        })
        .collect())
}

pub(crate) fn draft_audio_paths_for_cleanup(
    conn: &Connection,
    button_id: i64,
    language: &str,
) -> Result<Vec<String>> {
    conn.prepare(
        "select audio_path \
         from content_items \
         where source in ('recorded', 'uploaded', 'generated') and state = 'archived' \
           and content_type = 'language' and button_id = ?1 and language = ?2 \
           and audio_path is not null",
    )?
    .query_map(params![button_id, language], |row| row.get(0))?
    .collect::<rusqlite::Result<Vec<_>>>()
    .context("failed to read draft audio paths for cleanup")
}

pub(crate) fn content_item_by_id(
    conn: &Connection,
    item_id: &str,
) -> Result<Option<ContentItemRow>> {
    conn.prepare(
        "select id, content_type, title, text, language, source, state, audio_path \
         from content_items where id = ?1",
    )?
    .query_row([item_id], content_item_from_row)
    .optional()
    .context("failed to read content item")
}

pub(crate) fn next_order_index(
    conn: &Connection,
    content_type: &str,
    button_id: i64,
) -> Result<i64> {
    conn.query_row(
        "select coalesce(max(order_index), -1) + 1 from content_items where content_type = ?1 and button_id = ?2",
        params![content_type, button_id],
        |row| row.get(0),
    )
    .context("failed to allocate content order")
}

pub(crate) fn insert_media_artifact_if_present(
    conn: &Connection,
    item_id: &str,
    source: &str,
    path: &str,
    text: Option<&str>,
) -> Result<()> {
    if !table_exists(conn, "media_artifacts")? {
        return Ok(());
    }
    let media_type = match source {
        "recorded" => "recorded_audio",
        "uploaded" => "uploaded_audio",
        "generated" => "tts_audio",
        _ => "uploaded_audio",
    };
    conn.execute(
        "insert into media_artifacts (id, content_item_id, media_type, path, text, state) \
         values (?1, ?2, ?3, ?4, ?5, 'active')",
        params![
            format!("media-{}", random_token(12)?),
            item_id,
            media_type,
            path,
            text
        ],
    )?;
    Ok(())
}

fn content_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContentItemRow> {
    Ok(ContentItemRow {
        id: row.get(0)?,
        content_type: row.get(1)?,
        title: row.get(2)?,
        text: row.get(3)?,
        language: row.get(4)?,
        source: row.get(5)?,
        state: row.get(6)?,
        audio_path: row.get(7)?,
    })
}

fn inventory_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContentInventoryRow> {
    Ok(ContentInventoryRow {
        id: row.get(0)?,
        content_type: row.get(1)?,
        title: row.get(2)?,
        text: row.get(3)?,
        language: row.get(4)?,
        source: row.get(5)?,
        state: row.get(6)?,
        audio_path: row.get(7)?,
        button_id: row.get(8)?,
    })
}

fn empty_state_no_content(
    content_type: &str,
    language: Option<&str>,
    state: &str,
) -> ContentEmptyState {
    ContentEmptyState {
        title: empty_title(content_type, language, state),
        detail: format!(
            "No {} {} content exists yet. Record, upload, or generate content to create the first item.",
            language.unwrap_or_else(|| content_type_label(content_type)),
            state_label(state)
        ),
    }
}

fn empty_title(content_type: &str, language: Option<&str>, state: &str) -> String {
    let scope = language.unwrap_or_else(|| content_type_label(content_type));
    format!("No {} {} content on this button", state_label(state), scope)
}

fn state_label(state: &str) -> &'static str {
    if state == "active" {
        "active"
    } else {
        "draft"
    }
}

fn content_type_label(content_type: &str) -> &'static str {
    match content_type {
        "language" => "language",
        "animals" => "animal",
        "music" => "music",
        _ => "content",
    }
}

fn join_labels(values: &[String]) -> String {
    values
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_button_labels(buttons: &[i64]) -> String {
    buttons
        .iter()
        .take(3)
        .map(|button| format!("Button {button}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn count_content_rows(
    conn: &Connection,
    content_type: &str,
    language: Option<&str>,
    state: &str,
) -> Result<i64> {
    let source_filter = if state == "archived" {
        " and source in ('recorded', 'uploaded', 'generated')"
    } else {
        ""
    };
    let sql = if content_type == "language" && language.is_some() {
        format!(
            "select count(*) from content_items where content_type = ?1 and state = ?2 and language = ?3{source_filter}"
        )
    } else {
        format!(
            "select count(*) from content_items where content_type = ?1 and state = ?2{source_filter}"
        )
    };
    if content_type == "language" {
        if let Some(language) = language {
            return conn
                .query_row(&sql, params![content_type, state, language], |row| {
                    row.get(0)
                })
                .context("failed to count scoped content rows");
        }
    }
    conn.query_row(&sql, params![content_type, state], |row| row.get(0))
        .context("failed to count scoped content rows")
}

fn content_languages_for_button(
    conn: &Connection,
    button_id: i64,
    state: &str,
    excluded_language: Option<&str>,
) -> Result<Vec<String>> {
    let source_filter = if state == "archived" {
        " and source in ('recorded', 'uploaded', 'generated')"
    } else {
        ""
    };
    let sql = format!(
        "select distinct language from content_items \
         where button_id = ?1 and content_type = 'language' and state = ?2 \
           and language is not null{source_filter} \
         order by language"
    );
    let mut values = conn
        .prepare(&sql)?
        .query_map(params![button_id, state], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read content languages for button")?;
    if let Some(excluded_language) = excluded_language {
        values.retain(|value| value != excluded_language);
    }
    Ok(values)
}

fn content_buttons_for_language(
    conn: &Connection,
    button_id: i64,
    language: &str,
    state: &str,
) -> Result<Vec<i64>> {
    let source_filter = if state == "archived" {
        " and source in ('recorded', 'uploaded', 'generated')"
    } else {
        ""
    };
    let sql = format!(
        "select distinct button_id from content_items \
         where button_id != ?1 and content_type = 'language' and state = ?2 and language = ?3{source_filter} \
         order by button_id"
    );
    conn.prepare(&sql)?
        .query_map(params![button_id, state, language], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read content buttons for language")
}

fn content_buttons_for_type(
    conn: &Connection,
    button_id: i64,
    content_type: &str,
    state: &str,
) -> Result<Vec<i64>> {
    let source_filter = if state == "archived" {
        " and source in ('recorded', 'uploaded', 'generated')"
    } else {
        ""
    };
    let sql = format!(
        "select distinct button_id from content_items \
         where button_id != ?1 and content_type = ?2 and state = ?3{source_filter} \
         order by button_id"
    );
    conn.prepare(&sql)?
        .query_map(params![button_id, content_type, state], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read content buttons for type")
}

fn current_button_mappings(conn: &Connection) -> Result<HashMap<i64, CurrentButtonMapping>> {
    let mut mappings = default_button_mappings();
    if !table_exists(conn, "button_mappings")? {
        return Ok(mappings);
    }

    let mut stmt = conn.prepare("select button_id, mode, language from button_mappings")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            CurrentButtonMapping {
                mode: row.get(1)?,
                language: row.get(2)?,
            },
        ))
    })?;
    for row in rows {
        let (button_id, mapping) = row?;
        mappings.insert(button_id, mapping);
    }
    Ok(mappings)
}

fn default_button_mappings() -> HashMap<i64, CurrentButtonMapping> {
    HashMap::from([
        (
            1,
            CurrentButtonMapping {
                mode: "language".to_string(),
                language: Some("English".to_string()),
            },
        ),
        (
            2,
            CurrentButtonMapping {
                mode: "animals".to_string(),
                language: None,
            },
        ),
        (
            3,
            CurrentButtonMapping {
                mode: "music".to_string(),
                language: None,
            },
        ),
        (
            4,
            CurrentButtonMapping {
                mode: "setup_help".to_string(),
                language: None,
            },
        ),
        (
            5,
            CurrentButtonMapping {
                mode: "setup_help".to_string(),
                language: None,
            },
        ),
    ])
}

fn inventory_status_for_mapping(
    item: &ContentInventoryRow,
    mapping: Option<&CurrentButtonMapping>,
) -> (&'static str, String) {
    if item.state == "archived" {
        return (
            "draft",
            "Draft audio is waiting for review and activation.".to_string(),
        );
    }

    let Some(mapping) = mapping else {
        return (
            "unused",
            format!("Button {} has no current mode mapping.", item.button_id),
        );
    };

    if mapping.mode != item.content_type {
        return (
            "unused",
            format!(
                "Button {} is set to {}, not {}.",
                item.button_id,
                mode_reason_label(&mapping.mode),
                content_type_label(&item.content_type)
            ),
        );
    }

    if item.content_type == "language" {
        let selected = mapping.language.as_deref().unwrap_or("English");
        let content_language = item.language.as_deref().unwrap_or("unknown");
        if selected != content_language {
            return (
                "unused",
                format!(
                    "Button {} is set to {selected}, not {content_language}.",
                    item.button_id
                ),
            );
        }
    }

    ("active", "Active in the current button setup.".to_string())
}

fn mode_reason_label(mode: &str) -> &str {
    match mode {
        "setup_help" => "setup help",
        "disabled" => "disabled",
        value => value,
    }
}
