use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::config::AdminConfig;
use crate::db::admin::auth::{
    authenticate_session, now, random_token, require_local_cube_role, timestamp, RoleRequirement,
};
use crate::db::admin::content::{self as content_storage, ContentEmptyState, NewContentItem};
use crate::server::media::{
    activate_audio_file, content_preview_url, delete_content_audio_file, delete_draft_audio_file,
    delete_draft_audio_files, draft_audio_path, generated_filename, inspect_wav, media_filename,
    normalize_media_input, uploaded_audio_extension, validate_wav, write_draft_audio_file,
    MediaInput, MAX_AUDIO_BYTES,
};
use crate::server::speech::{
    generate_speech_audio, generated_speech_status_response, GeneratedSpeechStatusResponse,
};

#[derive(Debug, Serialize)]
pub(crate) struct ActiveContentResponse {
    id: String,
    content_type: String,
    title: String,
    text: String,
    source: String,
    state: &'static str,
    audio_path: Option<String>,
    preview_url: Option<String>,
    play_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct InactiveContentResponse {
    id: String,
    content_type: String,
    title: String,
    text: Option<String>,
    language: Option<String>,
    state: &'static str,
    source: String,
    audio_path: String,
    preview_url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentEmptyStateResponse {
    title: String,
    detail: String,
}

impl From<ContentEmptyState> for ContentEmptyStateResponse {
    fn from(empty_state: ContentEmptyState) -> Self {
        Self {
            title: empty_state.title,
            detail: empty_state.detail,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentListResponse<T> {
    items: Vec<T>,
    empty_state: Option<ContentEmptyStateResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentInventoryResponse {
    items: Vec<ContentInventoryItemResponse>,
    active_count: usize,
    draft_count: usize,
    unused_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentInventoryItemResponse {
    id: String,
    status: String,
    button_id: i64,
    content_type: String,
    language: Option<String>,
    title: String,
    text: Option<String>,
    source: String,
    state: String,
    audio_path: Option<String>,
    preview_url: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CleanupResponse {
    status: &'static str,
    deleted_count: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedCleanupRequest {
    button_id: i64,
    language: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedSpeechRequest {
    button_id: i64,
    language: String,
    text: String,
    provider: Option<String>,
    voice: Option<String>,
}

pub(crate) fn list_active_content(
    config: &AdminConfig,
    token: Option<&str>,
    button_id: i64,
    content_type: &str,
    language: Option<&str>,
) -> Result<ContentListResponse<ActiveContentResponse>> {
    let conn = authenticated_connection(config, token)?;
    validate_content_scope(button_id, content_type)?;
    let language = language.map(str::trim).filter(|value| !value.is_empty());
    let items = content_storage::active_content_rows(&conn, button_id, content_type, language)?;
    let empty_state = if items.is_empty() {
        content_storage::content_empty_state(&conn, button_id, content_type, language, "active")?
            .map(Into::into)
    } else {
        None
    };
    let items = items
        .into_iter()
        .map(|item| {
            let title = item.title.unwrap_or_else(|| item.id.clone());
            let text = item.text.unwrap_or_else(|| title.clone());
            ActiveContentResponse {
                id: item.id,
                content_type: item.content_type,
                title,
                text,
                source: item.source,
                state: "active",
                preview_url: item.audio_path.as_deref().map(content_preview_url),
                audio_path: item.audio_path,
                play_count: item.play_count,
            }
        })
        .collect();
    Ok(ContentListResponse { items, empty_state })
}

pub(crate) fn list_inactive_content(
    config: &AdminConfig,
    token: Option<&str>,
    button_id: i64,
    content_type: &str,
    language: Option<&str>,
) -> Result<ContentListResponse<InactiveContentResponse>> {
    let conn = authenticated_connection(config, token)?;
    validate_content_scope(button_id, content_type)?;
    let language = if content_type == "language" {
        language.map(str::trim).filter(|value| !value.is_empty())
    } else {
        None
    };
    let rows = content_storage::inactive_content_rows(&conn, button_id, content_type, language)?;
    let empty_state = if rows.is_empty() {
        content_storage::content_empty_state(&conn, button_id, content_type, language, "archived")?
            .map(Into::into)
    } else {
        None
    };
    let items = rows
        .into_iter()
        .map(|item| {
            let title = item.title.unwrap_or_else(|| item.id.clone());
            InactiveContentResponse {
                id: item.id,
                content_type: item.content_type.clone(),
                title: title.clone(),
                text: if item.content_type == "music" {
                    None
                } else {
                    Some(item.text.unwrap_or_else(|| title.clone()))
                },
                language: item.language,
                state: "archived",
                source: item.source,
                preview_url: content_preview_url(&item.audio_path.clone().unwrap_or_default()),
                audio_path: item.audio_path.unwrap_or_default(),
            }
        })
        .collect();
    Ok(ContentListResponse { items, empty_state })
}

pub(crate) fn content_inventory(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<ContentInventoryResponse> {
    let conn = authenticated_connection(config, token)?;
    let rows = content_storage::content_inventory_rows(&conn)?;
    let mappings = content_storage::current_button_mappings(&conn)?;
    let mut active_count = 0;
    let mut draft_count = 0;
    let mut unused_count = 0;
    let items = rows
        .into_iter()
        .map(|item| {
            let (status, reason) =
                content_storage::inventory_status_for_mappings(&item, &mappings)?;
            match status {
                "active" => active_count += 1,
                "draft" => draft_count += 1,
                "unused" => unused_count += 1,
                _ => {}
            }
            Ok(ContentInventoryItemResponse {
                id: item.id,
                status: status.to_string(),
                button_id: item.button_id,
                content_type: item.content_type,
                language: item.language,
                title: item.title.unwrap_or_else(|| "Untitled audio".to_string()),
                text: item.text,
                source: item.source,
                state: item.state,
                preview_url: item.audio_path.as_deref().map(content_preview_url),
                audio_path: item.audio_path,
                reason,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ContentInventoryResponse {
        items,
        active_count,
        draft_count,
        unused_count,
    })
}

pub(crate) fn trash_unused_content(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<CleanupResponse> {
    let conn = owner_connection(config, token)?;
    let unused = content_storage::unused_content_items(&conn)?;
    let audio_paths = unused
        .iter()
        .filter_map(|item| item.audio_path.clone())
        .collect::<Vec<_>>();
    for audio_path in &audio_paths {
        delete_content_audio_file(config, audio_path)?;
    }
    let trashed_at = now();
    let purge_after_at = purge_after();
    content_storage::trash_content_items(&conn, &unused, &trashed_at, &purge_after_at)?;
    Ok(CleanupResponse {
        status: "ok",
        deleted_count: unused.len(),
    })
}

pub(crate) fn save_multipart_media(
    config: &AdminConfig,
    token: Option<&str>,
    input: MediaInput,
    source: &str,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, token)?;
    let normalized = normalize_media_input(&input, source)?;
    if input.audio_bytes.len() > MAX_AUDIO_BYTES {
        anyhow::bail!("{source} audio must be 25 MB or smaller");
    }
    let extension = if source == "recorded" {
        let wav = inspect_wav(&input.audio_bytes)?;
        validate_wav(&wav, &normalized.content_type)?;
        "wav"
    } else {
        let extension = uploaded_audio_extension(&input.original_filename, &input.mime_type)?;
        if extension == "wav" {
            let wav = inspect_wav(&input.audio_bytes)?;
            validate_wav(&wav, &normalized.content_type)?;
        }
        extension
    };
    let filename = media_filename(
        source,
        &normalized.content_type,
        &normalized.language,
        if normalized.content_type == "language" {
            &normalized.text
        } else {
            &normalized.title
        },
        extension,
    );
    let title = if normalized.content_type == "language" {
        filename.clone()
    } else {
        normalized.title.clone()
    };
    let relative_path = draft_audio_path(&normalized.content_type, &filename);
    write_draft_audio_file(
        config,
        &normalized.content_type,
        &filename,
        &input.audio_bytes,
    )?;

    let item_id = format!("{source}-{}-{}", normalized.content_type, random_token(12)?);
    let order_index =
        content_storage::next_order_index(&conn, &normalized.content_type, normalized.button_id)?;
    let text = if normalized.content_type == "language" {
        normalized.text.clone()
    } else {
        normalized.title.clone()
    };
    content_storage::insert_content_item(
        &conn,
        &NewContentItem {
            id: &item_id,
            content_type: &normalized.content_type,
            button_id: normalized.button_id,
            language: empty_to_null(&normalized.language),
            title: &title,
            text: &text,
            audio_path: &relative_path,
            source,
            order_index,
        },
    )?;
    content_storage::insert_media_artifact_if_present(
        &conn,
        &item_id,
        source,
        &relative_path,
        None,
    )?;
    inactive_response_for_item(&conn, &item_id)
}

pub(crate) fn save_generated_speech(
    config: &AdminConfig,
    token: Option<&str>,
    body: GeneratedSpeechRequest,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, token)?;
    let text = body.text.trim();
    let language = body.language.trim();
    if !(1..=5).contains(&body.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    if text.is_empty() {
        anyhow::bail!("generated speech text is required");
    }
    if text.len() > 240 {
        anyhow::bail!("generated speech text must be 240 characters or fewer");
    }
    if language.is_empty() {
        anyhow::bail!("language is required");
    }
    let generated = generate_speech_audio(
        body.provider.as_deref().unwrap_or("auto"),
        language,
        text,
        body.voice.as_deref(),
    )?;
    if generated.bytes.len() > MAX_AUDIO_BYTES {
        anyhow::bail!("generated audio must be 25 MB or smaller");
    }
    if generated.extension == "wav" {
        let wav = inspect_wav(&generated.bytes)?;
        validate_wav(&wav, "language")?;
    }
    let filename = generated_filename(&generated.model, language, text, generated.extension);
    let relative_path = draft_audio_path("language", &filename);
    write_draft_audio_file(config, "language", &filename, &generated.bytes)?;

    let item_id = format!("generated-language-{}", random_token(12)?);
    let order_index = content_storage::next_order_index(&conn, "language", body.button_id)?;
    content_storage::insert_content_item(
        &conn,
        &NewContentItem {
            id: &item_id,
            content_type: "language",
            button_id: body.button_id,
            language: Some(language),
            title: &filename,
            text,
            audio_path: &relative_path,
            source: "generated",
            order_index,
        },
    )?;
    content_storage::insert_media_artifact_if_present(
        &conn,
        &item_id,
        "generated",
        &relative_path,
        Some(text),
    )?;
    inactive_response_for_item(&conn, &item_id)
}

pub(crate) fn generated_speech_status(
    config: &AdminConfig,
    token: Option<&str>,
    provider: &str,
    language: &str,
) -> Result<GeneratedSpeechStatusResponse> {
    let _conn = authenticated_connection(config, token)?;
    generated_speech_status_response(provider.trim(), language.trim())
}

pub(crate) fn activate_content_item(
    config: &AdminConfig,
    token: Option<&str>,
    item_id: &str,
) -> Result<InactiveContentResponse> {
    let conn = authenticated_connection(config, token)?;
    let item =
        content_storage::content_item_by_id(&conn, item_id)?.context("content item not found")?;
    if !matches!(item.source.as_str(), "recorded" | "uploaded" | "generated") {
        anyhow::bail!(
            "only inactive recorded, uploaded, or generated content can be activated here"
        );
    }
    if item.state != "archived" {
        anyhow::bail!("content is not inactive");
    }
    let audio_path = item.audio_path.clone().unwrap_or_default();
    let next_audio_path = activate_audio_file(config, &audio_path)?;
    content_storage::activate_content_item(&conn, &item.id, &next_audio_path)?;
    Ok(InactiveContentResponse {
        id: item.id,
        content_type: item.content_type.clone(),
        title: item.title.unwrap_or_else(|| item_id.to_string()),
        text: if item.content_type == "music" {
            None
        } else {
            item.text.or_else(|| Some(item_id.to_string()))
        },
        language: item.language,
        state: "active",
        source: item.source,
        preview_url: content_preview_url(&next_audio_path),
        audio_path: next_audio_path,
    })
}

pub(crate) fn trash_content_item(
    config: &AdminConfig,
    token: Option<&str>,
    item_id: &str,
) -> Result<()> {
    let conn = authenticated_connection(config, token)?;
    let item =
        content_storage::content_item_by_id(&conn, item_id)?.context("content item not found")?;
    delete_draft_audio_file(config, item.audio_path.as_deref())?;
    let trashed_at = now();
    let changes = content_storage::trash_content_item(&conn, item_id, &trashed_at, &purge_after())?;
    if changes == 0 {
        anyhow::bail!("content item not found: {item_id}");
    }
    Ok(())
}

pub(crate) fn trash_unused_generated_speech(
    config: &AdminConfig,
    token: Option<&str>,
    body: GeneratedCleanupRequest,
) -> Result<CleanupResponse> {
    let conn = authenticated_connection(config, token)?;
    if !(1..=5).contains(&body.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let language = body.language.trim();
    if language.is_empty() {
        anyhow::bail!("language is required");
    }
    let draft_paths =
        content_storage::draft_audio_paths_for_cleanup(&conn, body.button_id, language)?;
    delete_draft_audio_files(config, &draft_paths)?;
    let trashed_at = now();
    let deleted_count = content_storage::trash_generated_language_drafts(
        &conn,
        body.button_id,
        language,
        &trashed_at,
        &purge_after(),
    )?;
    Ok(CleanupResponse {
        status: "ok",
        deleted_count,
    })
}

pub(crate) fn inactive_response_for_item(
    conn: &Connection,
    item_id: &str,
) -> Result<InactiveContentResponse> {
    let item =
        content_storage::content_item_by_id(conn, item_id)?.context("content item not found")?;
    let title = item.title.unwrap_or_else(|| item.id.clone());
    let audio_path = item.audio_path.unwrap_or_default();
    Ok(InactiveContentResponse {
        id: item.id,
        content_type: item.content_type.clone(),
        title: title.clone(),
        text: if item.content_type == "music" {
            None
        } else {
            Some(item.text.unwrap_or_else(|| title.clone()))
        },
        language: item.language,
        state: if item.state == "active" {
            "active"
        } else {
            "archived"
        },
        source: item.source,
        preview_url: content_preview_url(&audio_path),
        audio_path,
    })
}

fn authenticated_connection(config: &AdminConfig, token: Option<&str>) -> Result<Connection> {
    role_authorized_connection(config, token, RoleRequirement::Member)
}

fn owner_connection(config: &AdminConfig, token: Option<&str>) -> Result<Connection> {
    role_authorized_connection(config, token, RoleRequirement::Owner)
}

fn role_authorized_connection(
    config: &AdminConfig,
    token: Option<&str>,
    requirement: RoleRequirement,
) -> Result<Connection> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    require_local_cube_role(&conn, &session.account.id, requirement)?;
    Ok(conn)
}

fn validate_content_scope(button_id: i64, content_type: &str) -> Result<()> {
    if !(1..=5).contains(&button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    if !matches!(content_type, "language" | "animals" | "music") {
        anyhow::bail!("unsupported content type");
    }
    Ok(())
}

fn purge_after() -> String {
    timestamp(Utc::now() + chrono::Duration::days(15))
}

fn empty_to_null(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
