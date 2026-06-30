use std::fs;

use anyhow::{Context, Result};
use axum::extract::multipart::Field;
use axum::extract::Multipart;
use chrono::Utc;

use crate::config::AdminConfig;
pub(crate) const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct MediaInput {
    pub(crate) content_type: String,
    pub(crate) button_id: i64,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) language: String,
    pub(crate) audio_bytes: Vec<u8>,
    pub(crate) original_filename: String,
    pub(crate) mime_type: String,
}

#[derive(Debug)]
pub(crate) struct NormalizedMediaInput {
    pub(crate) content_type: String,
    pub(crate) button_id: i64,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) language: String,
}

#[derive(Debug)]
pub(crate) struct WavInspection {
    duration_seconds: f64,
    peak: f64,
    rms: f64,
}

pub(crate) async fn media_input_from_axum_multipart(
    mut multipart: Multipart,
) -> Result<MediaInput> {
    let mut content_type = String::new();
    let mut button_id = String::new();
    let mut title = String::new();
    let mut text = String::new();
    let mut language = String::new();
    let mut audio_bytes = None;
    let mut original_filename = None;
    let mut mime_type = None;

    while let Some(mut field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        if name == "audio_file" {
            original_filename = field.file_name().map(str::to_string);
            mime_type = field.content_type().map(str::to_string);
            audio_bytes = Some(read_limited_audio_field(&mut field).await?);
            continue;
        }
        let value = field.text().await?;
        match name.as_str() {
            "content_type" => content_type = value,
            "button_id" => button_id = value,
            "title" => title = value,
            "text" => text = value,
            "language" => language = value,
            _ => {}
        }
    }

    Ok(MediaInput {
        content_type,
        button_id: button_id
            .trim()
            .parse::<i64>()
            .context("button id must be between 1 and 5")?,
        title,
        text,
        language,
        audio_bytes: audio_bytes.context("audio file is required")?,
        original_filename: original_filename.unwrap_or_else(|| "upload".to_string()),
        mime_type: mime_type.unwrap_or_default(),
    })
}

async fn read_limited_audio_field(field: &mut Field<'_>) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await? {
        let next_len = bytes
            .len()
            .checked_add(chunk.len())
            .context("audio upload is too large")?;
        if next_len > MAX_AUDIO_BYTES {
            anyhow::bail!("audio file must be 25 MB or smaller");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(crate) fn normalize_media_input(
    input: &MediaInput,
    source: &str,
) -> Result<NormalizedMediaInput> {
    if !matches!(
        input.content_type.as_str(),
        "language" | "animals" | "music"
    ) {
        anyhow::bail!("unsupported {source} content type");
    }
    if !(1..=5).contains(&input.button_id) {
        anyhow::bail!("button id must be between 1 and 5");
    }
    let title = input.title.trim();
    let text = input.text.trim();
    let language = input.language.trim();
    if input.content_type == "language" && language.is_empty() {
        anyhow::bail!("language {source}s require language");
    }
    if input.content_type == "language" && text.is_empty() {
        anyhow::bail!("language {source}s require spoken text");
    }
    if input.content_type != "language" && title.is_empty() {
        anyhow::bail!("{source} title is required");
    }
    Ok(NormalizedMediaInput {
        content_type: input.content_type.clone(),
        button_id: input.button_id,
        title: title.to_string(),
        text: text.to_string(),
        language: language.to_string(),
    })
}

pub(crate) fn uploaded_audio_extension(filename: &str, mime_type: &str) -> Result<&'static str> {
    let filename = filename.to_ascii_lowercase();
    let mime_type = mime_type.to_ascii_lowercase();
    if filename.ends_with(".wav") || mime_type.contains("wav") {
        return Ok("wav");
    }
    if filename.ends_with(".mp3") || mime_type.contains("mpeg") || mime_type.contains("mp3") {
        return Ok("mp3");
    }
    anyhow::bail!("uploaded audio must be an MP3 or WAV file");
}

pub(crate) fn validate_wav(wav: &WavInspection, content_type: &str) -> Result<()> {
    let max_duration = if content_type == "music" { 180.0 } else { 15.0 };
    if wav.duration_seconds > max_duration {
        if content_type == "music" {
            anyhow::bail!("music audio must be 3 minutes or shorter");
        }
        anyhow::bail!("language and animal audio must be 15 seconds or shorter");
    }
    if wav.peak < 0.02 || wav.rms < 0.005 {
        anyhow::bail!("audio is too quiet");
    }
    Ok(())
}

pub(crate) fn inspect_wav(bytes: &[u8]) -> Result<WavInspection> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        anyhow::bail!("recorded audio must be a WAV file");
    }
    let mut offset = 12_usize;
    let mut audio_format = 0_u16;
    let mut channels = 0_u16;
    let mut sample_rate = 0_u32;
    let mut bits_per_sample = 0_u16;
    let mut data_offset = None;
    let mut data_size = 0_usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = read_le_u32(bytes, offset + 4)? as usize;
        let chunk_data_offset = offset + 8;
        let chunk_end = chunk_data_offset
            .checked_add(chunk_size)
            .context("recorded WAV file is malformed")?;
        if chunk_end > bytes.len() {
            anyhow::bail!("recorded WAV file is malformed");
        }
        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                anyhow::bail!("recorded WAV file is malformed");
            }
            audio_format = read_le_u16(bytes, chunk_data_offset)?;
            channels = read_le_u16(bytes, chunk_data_offset + 2)?;
            sample_rate = read_le_u32(bytes, chunk_data_offset + 4)?;
            bits_per_sample = read_le_u16(bytes, chunk_data_offset + 14)?;
        } else if chunk_id == b"data" {
            data_offset = Some(chunk_data_offset);
            data_size = chunk_size;
            break;
        }
        offset = chunk_end
            .checked_add(chunk_size % 2)
            .context("recorded WAV file is malformed")?;
    }
    if audio_format != 1 || bits_per_sample != 16 || channels < 1 || sample_rate < 8000 {
        anyhow::bail!("recorded WAV file must be 16-bit PCM audio");
    }
    let data_offset = data_offset.context("recorded WAV file has no audio data")?;
    if data_size < 2 {
        anyhow::bail!("recorded WAV file has no audio data");
    }
    let mut peak = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut samples = 0_usize;
    for sample_offset in (data_offset..data_offset + data_size - 1).step_by(2) {
        let sample = read_le_i16(bytes, sample_offset)? as f64 / 32768.0;
        let abs = sample.abs();
        peak = peak.max(abs);
        sum_squares += sample * sample;
        samples += 1;
    }
    Ok(WavInspection {
        duration_seconds: samples as f64 / channels as f64 / sample_rate as f64,
        peak,
        rms: (sum_squares / samples as f64).sqrt(),
    })
}

fn read_le_i16(bytes: &[u8], offset: usize) -> Result<i16> {
    let slice = bytes
        .get(offset..offset + 2)
        .context("recorded WAV file is malformed")?;
    Ok(i16::from_le_bytes([slice[0], slice[1]]))
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .context("recorded WAV file is malformed")?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .context("recorded WAV file is malformed")?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub(crate) fn media_filename(
    source: &str,
    content_type: &str,
    language: &str,
    label: &str,
    extension: &str,
) -> String {
    if content_type == "language" {
        format!(
            "{source}-{}-{}-{}.{}",
            slug_part(language),
            slug_part(label),
            recording_timestamp(),
            if source == "recorded" {
                "wav"
            } else {
                extension
            }
        )
    } else {
        format!(
            "{}-{}-{}.{}",
            slug_part(source),
            slug_part(label),
            recording_timestamp(),
            if source == "recorded" {
                "wav"
            } else {
                extension
            }
        )
    }
}

pub(crate) fn generated_filename(
    model: &str,
    language: &str,
    text: &str,
    extension: &str,
) -> String {
    let text_slug = slug_part(text);
    let truncated = text_slug.chars().take(72).collect::<String>();
    format!(
        "generated-{}-{}-{}-{}.{}",
        slug_part(model),
        slug_part(language),
        truncated,
        recording_timestamp(),
        extension
    )
}

pub(crate) fn draft_audio_path(content_type: &str, filename: &str) -> String {
    format!("data/audio/draft/{content_type}/{filename}")
}

pub(crate) fn write_draft_audio_file(
    config: &AdminConfig,
    content_type: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<()> {
    let absolute_path = config
        .media_root
        .join("draft")
        .join(content_type)
        .join(filename);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create media directory {}", parent.display()))?;
    }
    fs::write(&absolute_path, bytes)
        .with_context(|| format!("failed to write media file {}", absolute_path.display()))
}

pub(crate) fn content_preview_url(audio_path: &str) -> String {
    audio_path
        .strip_prefix("data/audio/")
        .map(|path| format!("/api/media/{path}"))
        .or_else(|| {
            audio_path
                .strip_prefix("data/media/")
                .map(|path| format!("/api/media/{path}"))
        })
        .unwrap_or_else(|| format!("/{audio_path}"))
}

pub(crate) fn activate_audio_file(config: &AdminConfig, audio_path: &str) -> Result<String> {
    let Some(active_path) = active_audio_path_from_draft(audio_path) else {
        return Ok(audio_path.to_string());
    };
    let draft_relative = audio_path
        .strip_prefix("data/audio/")
        .context("draft audio path must be under data/audio")?;
    let active_relative = active_path
        .strip_prefix("data/audio/")
        .context("active audio path must be under data/audio")?;
    let draft_absolute = config.media_root.join(draft_relative);
    let active_absolute = config.media_root.join(active_relative);
    if let Some(parent) = active_absolute.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create active audio directory {}",
                parent.display()
            )
        })?;
    }
    fs::rename(&draft_absolute, &active_absolute).with_context(|| {
        format!(
            "failed to move draft audio {} to active audio {}",
            draft_absolute.display(),
            active_absolute.display()
        )
    })?;
    Ok(active_path)
}

pub(crate) fn delete_draft_audio_file(
    config: &AdminConfig,
    audio_path: Option<&str>,
) -> Result<()> {
    let Some(audio_path) = audio_path else {
        return Ok(());
    };
    let Some(path) = draft_audio_absolute_path(config, audio_path) else {
        return Ok(());
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to delete draft audio file {}", path.display())),
    }
}

pub(crate) fn delete_draft_audio_files(config: &AdminConfig, audio_paths: &[String]) -> Result<()> {
    for audio_path in audio_paths {
        delete_draft_audio_file(config, Some(audio_path))?;
    }
    Ok(())
}

pub(crate) fn delete_content_audio_file(config: &AdminConfig, audio_path: &str) -> Result<()> {
    let Some(path) = content_audio_absolute_path(config, audio_path) else {
        return Ok(());
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to delete audio file {}", path.display()))
        }
    }
}

fn slug_part(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "unknown".to_string()
    } else {
        slug
    }
}

fn recording_timestamp() -> String {
    Utc::now().format("%Y%m%d%H%M%S%3f").to_string()
}

fn active_audio_path_from_draft(audio_path: &str) -> Option<String> {
    audio_path
        .strip_prefix("data/audio/draft/")
        .map(|path| format!("data/audio/active/{path}"))
}

fn draft_audio_absolute_path(config: &AdminConfig, audio_path: &str) -> Option<std::path::PathBuf> {
    let draft_relative = audio_path.strip_prefix("data/audio/draft/")?;
    Some(config.media_root.join("draft").join(draft_relative))
}

fn content_audio_absolute_path(
    config: &AdminConfig,
    audio_path: &str,
) -> Option<std::path::PathBuf> {
    let relative = audio_path.strip_prefix("data/audio/")?;
    Some(config.media_root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::{inspect_wav, validate_wav};

    #[test]
    fn inspect_wav_accepts_valid_pcm_audio() {
        let wav = wav_with_samples(8_000, 800, 10_000, 1, 16);

        let inspection = inspect_wav(&wav).expect("valid wav should inspect");

        validate_wav(&inspection, "language").expect("valid wav should pass product validation");
    }

    #[test]
    fn inspect_wav_rejects_short_fmt_chunk_without_panicking() {
        let mut wav = riff_header();
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(&[1, 0, 1, 0]);
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2_u32.to_le_bytes());
        wav.extend_from_slice(&10_000_i16.to_le_bytes());
        wav.extend_from_slice(&[0; 10]);

        let error = inspect_wav(&wav).expect_err("short fmt chunk should fail");

        assert_eq!(error.to_string(), "recorded WAV file is malformed");
    }

    #[test]
    fn inspect_wav_rejects_truncated_fmt_chunk_without_panicking() {
        let mut wav = riff_header();
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&100_u32.to_le_bytes());
        wav.extend_from_slice(&[0; 32]);

        let error = inspect_wav(&wav).expect_err("truncated fmt chunk should fail");

        assert_eq!(error.to_string(), "recorded WAV file is malformed");
    }

    #[test]
    fn inspect_wav_rejects_missing_data_chunk() {
        let mut wav = riff_header();
        append_fmt_chunk(&mut wav, 1, 1, 8_000, 16);
        wav.extend_from_slice(b"JUNK");
        wav.extend_from_slice(&0_u32.to_le_bytes());

        let error = inspect_wav(&wav).expect_err("missing data chunk should fail");

        assert_eq!(error.to_string(), "recorded WAV file has no audio data");
    }

    #[test]
    fn inspect_wav_rejects_non_pcm_audio() {
        let wav = wav_with_samples(8_000, 800, 10_000, 3, 16);

        let error = inspect_wav(&wav).expect_err("non-pcm wav should fail");

        assert_eq!(
            error.to_string(),
            "recorded WAV file must be 16-bit PCM audio"
        );
    }

    #[test]
    fn inspect_wav_rejects_non_sixteen_bit_audio() {
        let wav = wav_with_samples(8_000, 800, 10_000, 1, 8);

        let error = inspect_wav(&wav).expect_err("8-bit wav should fail");

        assert_eq!(
            error.to_string(),
            "recorded WAV file must be 16-bit PCM audio"
        );
    }

    #[test]
    fn validate_wav_rejects_quiet_audio() {
        let wav = wav_with_samples(8_000, 800, 1, 1, 16);
        let inspection = inspect_wav(&wav).expect("quiet wav should still inspect");

        let error = validate_wav(&inspection, "language").expect_err("quiet audio should fail");

        assert_eq!(error.to_string(), "audio is too quiet");
    }

    #[test]
    fn validate_wav_rejects_over_duration_language_audio() {
        let wav = wav_with_samples(8_000, 128_001, 10_000, 1, 16);
        let inspection = inspect_wav(&wav).expect("long wav should still inspect");

        let error =
            validate_wav(&inspection, "language").expect_err("over-duration audio should fail");

        assert_eq!(
            error.to_string(),
            "language and animal audio must be 15 seconds or shorter"
        );
    }

    fn riff_header() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes
    }

    fn append_fmt_chunk(
        bytes: &mut Vec<u8>,
        audio_format: u16,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
    ) {
        let bytes_per_sample = u32::from(bits_per_sample / 8);
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&audio_format.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(
            &(sample_rate * u32::from(channels) * bytes_per_sample).to_le_bytes(),
        );
        bytes.extend_from_slice(&(channels * (bits_per_sample / 8)).to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    }

    fn wav_with_samples(
        sample_rate: u32,
        sample_count: u32,
        amplitude: i16,
        audio_format: u16,
        bits_per_sample: u16,
    ) -> Vec<u8> {
        let mut bytes = riff_header();
        append_fmt_chunk(&mut bytes, audio_format, 1, sample_rate, bits_per_sample);
        let data_size = sample_count * 2;
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for index in 0..sample_count {
            let value = if index % 2 == 0 {
                amplitude
            } else {
                -amplitude
            };
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
