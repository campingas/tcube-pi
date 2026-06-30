use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

const SPEECH_PROVIDER_HEALTH_TTL: Duration = Duration::from_secs(20);
const SPEECH_PROVIDER_HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
static SPEECH_PROVIDER_HEALTH_CACHE: OnceLock<Mutex<HashMap<String, CachedSpeechProviderHealth>>> =
    OnceLock::new();

#[derive(Debug)]
pub(crate) struct GeneratedAudio {
    pub(crate) bytes: Vec<u8>,
    pub(crate) extension: &'static str,
    pub(crate) model: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GeneratedSpeechStatusResponse {
    online: bool,
    provider: String,
    checked_at: String,
    cached: bool,
    cache_ttl_seconds: u64,
    next_check_after_seconds: u64,
    message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedSpeechProviderHealth {
    pub(crate) online: bool,
    provider: String,
    checked_at: String,
    checked_instant: Instant,
    message: String,
}

pub(crate) fn generate_speech_audio(
    provider: &str,
    language: &str,
    text: &str,
    voice: Option<&str>,
) -> Result<GeneratedAudio> {
    let provider = resolve_speech_provider(provider, language);
    match provider {
        "voxtral" => {
            let base = speech_provider_base_url(provider)?;
            let model = std::env::var("VOXTRAL_MODEL")
                .unwrap_or_else(|_| "mistralai/Voxtral-4B-TTS-2603".to_string());
            let voice = voice
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| std::env::var("VOXTRAL_VOICE").ok())
                .unwrap_or_else(|| "neutral_male".to_string());
            let body = json!({
                "input": text,
                "model": model,
                "response_format": "wav",
                "voice": voice
            })
            .to_string();
            let bytes = post_speech_json(
                &format!("{}/audio/speech", base.trim_end_matches('/')),
                &body,
                vec![(
                    "Authorization".to_string(),
                    format!(
                        "Bearer {}",
                        std::env::var("VOXTRAL_API_KEY").unwrap_or_else(|_| "EMPTY".to_string())
                    ),
                )],
            )?;
            Ok(GeneratedAudio {
                bytes,
                extension: "wav",
                model: model_name_for_file(&model),
            })
        }
        "vietnamese-vits" => {
            let base = speech_provider_base_url(provider)?;
            let body = json!({ "input": text, "response_format": "wav" }).to_string();
            let bytes = post_speech_json(
                &format!("{}/v1/audio/speech", base.trim_end_matches('/')),
                &body,
                Vec::new(),
            )?;
            Ok(GeneratedAudio {
                bytes,
                extension: "wav",
                model: "vietnamese-vits".to_string(),
            })
        }
        "mistral" => {
            anyhow::bail!("hosted Mistral generation is not supported by the Pi Rust spike yet")
        }
        _ => anyhow::bail!("unsupported speech provider"),
    }
}

pub(crate) fn generated_speech_status_response(
    provider: &str,
    language: &str,
) -> Result<GeneratedSpeechStatusResponse> {
    let resolved_provider = resolve_speech_provider(provider, language);
    let base_url = speech_provider_base_url(resolved_provider)?;
    let cache_key = format!("{resolved_provider}:{base_url}");
    let cached = cached_speech_provider_health(cache_key, resolved_provider.to_string(), || {
        probe_speech_provider(&base_url)
    })?;
    Ok(speech_provider_status_response(cached))
}

pub(crate) fn validate_speech_api_url(url: &str) -> Result<reqwest::Url> {
    let parsed_url =
        reqwest::Url::parse(url).with_context(|| format!("invalid speech provider URL: {url}"))?;
    if parsed_url.scheme() != "https" {
        anyhow::bail!("speech provider URL must use https: {url}");
    }
    Ok(parsed_url)
}

pub(crate) fn speech_http_client_with_ca_cert_path(
    ca_cert_path: Option<&Path>,
) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(30));
    if let Some(path) = ca_cert_path {
        let pem = fs::read(path).with_context(|| {
            format!(
                "failed to read speech API CA certificate {}",
                path.display()
            )
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).with_context(|| {
            format!(
                "failed to parse speech API CA certificate {}",
                path.display()
            )
        })?;
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .context("failed to build speech API HTTP client")
}

fn resolve_speech_provider<'a>(provider: &'a str, language: &str) -> &'a str {
    if provider == "auto" {
        if language.eq_ignore_ascii_case("vietnamese") {
            "vietnamese-vits"
        } else {
            "voxtral"
        }
    } else {
        provider
    }
}

fn speech_provider_base_url(provider: &str) -> Result<String> {
    match provider {
        "voxtral" => Ok(std::env::var("VOXTRAL_API_BASE")
            .unwrap_or_else(|_| "https://127.0.0.1:8001/v1".to_string())),
        "vietnamese-vits" => Ok(std::env::var("VIETNAMESE_VITS_API_BASE")
            .unwrap_or_else(|_| "https://127.0.0.1:7872".to_string())),
        "mistral" => {
            anyhow::bail!("hosted Mistral generation is not supported by the Pi Rust spike yet")
        }
        _ => anyhow::bail!("unsupported speech provider"),
    }
}

pub(crate) fn cached_speech_provider_health(
    cache_key: String,
    provider: String,
    probe: impl FnOnce() -> Result<()>,
) -> Result<(CachedSpeechProviderHealth, bool)> {
    let cache = SPEECH_PROVIDER_HEALTH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache
        .lock()
        .expect("speech provider health cache poisoned")
        .get(&cache_key)
        .filter(|cached| cached.checked_instant.elapsed() < SPEECH_PROVIDER_HEALTH_TTL)
        .cloned()
    {
        return Ok((cached, true));
    }

    let checked_at = Utc::now().to_rfc3339();
    let (online, message) = match probe() {
        Ok(()) => (
            true,
            "TTS provider is online and ready for generated speech.".to_string(),
        ),
        Err(error) => (
            false,
            format!("TTS provider is offline or unreachable: {error}"),
        ),
    };
    let health = CachedSpeechProviderHealth {
        online,
        provider,
        checked_at,
        checked_instant: Instant::now(),
        message,
    };
    cache
        .lock()
        .expect("speech provider health cache poisoned")
        .insert(cache_key, health.clone());
    Ok((health, false))
}

fn speech_provider_status_response(
    health: (CachedSpeechProviderHealth, bool),
) -> GeneratedSpeechStatusResponse {
    let (health, cached) = health;
    GeneratedSpeechStatusResponse {
        online: health.online,
        provider: health.provider,
        checked_at: health.checked_at,
        cached,
        cache_ttl_seconds: SPEECH_PROVIDER_HEALTH_TTL.as_secs(),
        next_check_after_seconds: SPEECH_PROVIDER_HEALTH_TTL.as_secs(),
        message: health.message,
    }
}

pub(crate) fn probe_speech_provider(base_url: &str) -> Result<()> {
    let url = validate_speech_api_url(base_url)?;
    let client = speech_http_client_with_timeout(SPEECH_PROVIDER_HEALTH_TIMEOUT)?;
    client
        .get(url)
        .send()
        .with_context(|| format!("failed to connect to speech provider {base_url}"))?;
    Ok(())
}

fn post_speech_json(
    url: &str,
    body: &str,
    extra_headers: Vec<(String, String)>,
) -> Result<Vec<u8>> {
    let url_text = url.to_owned();
    let url = validate_speech_api_url(&url_text)?;
    let client = speech_http_client()?;
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_owned());
    for (name, value) in extra_headers {
        request = request.header(name, value);
    }

    let response = request
        .send()
        .with_context(|| format!("failed to connect to speech provider {url_text}"))?;
    let status = response.status();
    let response_body = response
        .bytes()
        .context("speech provider returned unreadable response body")?;
    if !status.is_success() {
        anyhow::bail!(
            "speech generation failed: {}",
            String::from_utf8_lossy(&response_body)
                .chars()
                .take(500)
                .collect::<String>()
        );
    }
    if response_body.is_empty() {
        anyhow::bail!("speech generation failed: empty audio response");
    }
    Ok(response_body.to_vec())
}

fn speech_http_client() -> Result<reqwest::blocking::Client> {
    speech_http_client_with_ca_cert_path(
        std::env::var_os("TCUBE_SPEECH_API_CA_CERT")
            .as_deref()
            .map(Path::new),
    )
}

fn speech_http_client_with_timeout(timeout: Duration) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout);
    if let Some(path) = std::env::var_os("TCUBE_SPEECH_API_CA_CERT")
        .as_deref()
        .map(Path::new)
    {
        let pem = fs::read(path).with_context(|| {
            format!(
                "failed to read speech API CA certificate {}",
                path.display()
            )
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).with_context(|| {
            format!(
                "failed to parse speech API CA certificate {}",
                path.display()
            )
        })?;
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .context("failed to build speech API HTTP client")
}

fn model_name_for_file(model: &str) -> String {
    model
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(model)
        .to_string()
}
