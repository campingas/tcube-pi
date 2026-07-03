use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::json;

use crate::db::measurements;
use crate::events::types::{ButtonBehavior, ButtonMapping, ContentPack};

#[derive(Clone, Debug)]
struct HttpTarget {
    host: String,
    port: u16,
}

#[derive(Debug)]
struct LatencyStats {
    count: usize,
    p50_us: u128,
    p95_us: u128,
    p99_us: u128,
    max_us: u128,
}

#[derive(Debug)]
pub struct MeasureConfig {
    pub base_url: String,
    pub content: PathBuf,
    pub button_presses: usize,
    pub admin_requests: usize,
    pub admin_workers: usize,
    pub database: Option<PathBuf>,
}

pub fn run(config: MeasureConfig) -> Result<()> {
    let content = load_content(&config.content)?;
    let database = config.database.unwrap_or_else(default_database_path);
    let target = parse_http_target(&config.base_url)?;

    if config.button_presses == 0 {
        bail!("--button-presses must be greater than zero");
    }
    if config.admin_workers == 0 {
        bail!("--admin-workers must be greater than zero");
    }

    let baseline = measure_button_latency(&content, &database, config.button_presses)
        .context("failed to measure baseline button latency")?;

    let load = spawn_admin_load(target, config.admin_requests, config.admin_workers);
    let loaded = measure_button_latency(&content, &database, config.button_presses)
        .context("failed to measure button latency under admin load")?;
    let load_result = load
        .join()
        .map_err(|_| anyhow::anyhow!("admin load worker panicked"))??;

    let report = json!({
        "status": "ok",
        "content": config.content,
        "database": database,
        "base_url": config.base_url,
        "button_presses": config.button_presses,
        "admin_requests": config.admin_requests,
        "admin_workers": config.admin_workers,
        "admin_load": load_result,
        "baseline": stats_json(&baseline),
        "under_admin_load": stats_json(&loaded),
        "delta_us": {
            "p50": loaded.p50_us.saturating_sub(baseline.p50_us),
            "p95": loaded.p95_us.saturating_sub(baseline.p95_us),
            "p99": loaded.p99_us.saturating_sub(baseline.p99_us),
            "max": loaded.max_us.saturating_sub(baseline.max_us)
        },
        "note": "Measures deterministic local selection plus SQLite event logging while concurrent HTTP requests hit the Pi admin service."
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn stats_json(stats: &LatencyStats) -> serde_json::Value {
    json!({
        "count": stats.count,
        "p50_us": stats.p50_us,
        "p95_us": stats.p95_us,
        "p99_us": stats.p99_us,
        "max_us": stats.max_us
    })
}

fn load_content(path: &Path) -> Result<ContentPack> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read content pack {}", path.display()))?;
    let content: ContentPack = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse content pack {}", path.display()))?;
    if content.modes.is_empty() {
        bail!("content pack has no modes");
    }
    Ok(content)
}

fn measure_button_latency(
    content: &ContentPack,
    database_path: &Path,
    button_presses: usize,
) -> Result<LatencyStats> {
    if let Some(parent) = database_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let conn = Connection::open(database_path)
        .with_context(|| format!("failed to open {}", database_path.display()))?;
    measurements::run_migrations(&conn)?;
    conn.execute("delete from button_events", [])?;
    conn.execute("delete from setup_debug_events", [])?;

    let active_buttons = active_button_mappings(content)?;
    let mut response_counts = HashMap::<String, usize>::new();
    let mut durations = Vec::with_capacity(button_presses);

    for index in 0..button_presses {
        let mapping = &active_buttons[index % active_buttons.len()];
        let started = Instant::now();
        handle_button_press(content, &conn, mapping, &mut response_counts)?;
        durations.push(started.elapsed().as_micros());
    }

    Ok(latency_stats(durations))
}

fn handle_button_press(
    content: &ContentPack,
    conn: &Connection,
    mapping: &ButtonMapping,
    response_counts: &mut HashMap<String, usize>,
) -> Result<()> {
    let mode = mapping
        .mode
        .as_ref()
        .with_context(|| format!("button {} is missing a mode", mapping.button_id))?;
    let count = response_counts.entry(mode.clone()).or_insert(0);
    let mode_content = content
        .modes
        .iter()
        .find(|item| item.mode == *mode)
        .with_context(|| format!("mode {mode} is missing"))?;
    if mode_content.responses.is_empty() {
        bail!("mode {mode} has no responses");
    }
    let response = &mode_content.responses[*count % mode_content.responses.len()];
    *count += 1;

    if content.setup_complete {
        conn.execute(
            "insert into button_events \
             (occurred_at, button_id, mode, response_id, response_text) values (?1, ?2, ?3, ?4, ?5)",
            params![
                Utc::now().to_rfc3339(),
                mapping.button_id,
                mode,
                response.id,
                response.text
            ],
        )?;
    } else {
        conn.execute(
            "insert into setup_debug_events (event_type, button_id, details) values (?1, ?2, ?3)",
            params![
                "first_run_button_press",
                mapping.button_id,
                format!(
                    "{{\"mode\":\"{}\",\"response_id\":\"{}\"}}",
                    mode, response.id
                )
            ],
        )?;
    }

    Ok(())
}

fn active_button_mappings(content: &ContentPack) -> Result<Vec<ButtonMapping>> {
    let mappings = content
        .button_mappings
        .iter()
        .filter(|mapping| {
            matches!(
                mapping.behavior,
                ButtonBehavior::Language
                    | ButtonBehavior::Animals
                    | ButtonBehavior::Music
                    | ButtonBehavior::Soundbox
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if mappings.is_empty() {
        bail!("content pack has no child-facing active button mappings");
    }
    Ok(mappings)
}

fn latency_stats(mut durations: Vec<u128>) -> LatencyStats {
    durations.sort_unstable();
    LatencyStats {
        count: durations.len(),
        p50_us: percentile(&durations, 50),
        p95_us: percentile(&durations, 95),
        p99_us: percentile(&durations, 99),
        max_us: *durations.last().unwrap_or(&0),
    }
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) * percentile).div_ceil(100);
    sorted[index]
}

fn spawn_admin_load(
    target: HttpTarget,
    request_count: usize,
    worker_count: usize,
) -> thread::JoinHandle<Result<serde_json::Value>> {
    thread::spawn(move || {
        let paths = [
            "/api/pi/v1/status",
            "/api/auth/session",
            "/api/setup/review",
            "/",
        ];
        let next = Arc::new(AtomicUsize::new(0));
        let success = Arc::new(AtomicUsize::new(0));
        let failure = Arc::new(AtomicUsize::new(0));
        let started = Instant::now();
        let mut handles = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let target = target.clone();
            let next = Arc::clone(&next);
            let success = Arc::clone(&success);
            let failure = Arc::clone(&failure);
            handles.push(thread::spawn(move || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= request_count {
                    break;
                }
                let path = paths[index % paths.len()];
                match http_get(&target, path) {
                    Ok(()) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failure.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }

        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("admin load worker panicked"))?;
        }

        Ok(json!({
            "duration_ms": started.elapsed().as_millis(),
            "success": success.load(Ordering::Relaxed),
            "failure": failure.load(Ordering::Relaxed)
        }))
    })
}

fn http_get(target: &HttpTarget, path: &str) -> Result<()> {
    let mut stream = TcpStream::connect((target.host.as_str(), target.port))
        .with_context(|| format!("failed to connect to {}:{}", target.host, target.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        target.host
    )?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    if status_line.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        bail!("unexpected HTTP response")
    }
}

fn parse_http_target(base_url: &str) -> Result<HttpTarget> {
    let rest = base_url
        .strip_prefix("http://")
        .context("only http:// URLs are supported by the measurement harness")?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host.to_string(), port.parse::<u16>()?),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        bail!("base URL host is empty");
    }
    Ok(HttpTarget { host, port })
}

fn default_database_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "tcube-pi-admin-measure-{}.sqlite3",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_targets() {
        let target = parse_http_target("http://127.0.0.1:8443").unwrap();
        assert_eq!(target.host, "127.0.0.1");
        assert_eq!(target.port, 8443);
    }

    #[test]
    fn calculates_percentiles() {
        let stats = latency_stats(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        assert_eq!(stats.p50_us, 6);
        assert_eq!(stats.p95_us, 10);
        assert_eq!(stats.p99_us, 10);
        assert_eq!(stats.max_us, 10);
    }
}
