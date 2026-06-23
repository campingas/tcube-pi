use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{routing::any, Router};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::config::DeviceApiConfig;

const DEVICE_TOKEN_HEADER: &str = "authorization";
const DEVICE_ID_HEADER: &str = "x-tcube-device-id";
const MAX_REQUEST_BODY_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct LatestPackageResponse {
    schema_version: i64,
    revision: i64,
    package_id: String,
    created_at: String,
    archive_size: i64,
    archive_sha256: String,
    minimum_runtime_version: String,
    download_url: String,
}

#[derive(Debug, Serialize)]
struct PackageAcknowledgementResponse {
    status: &'static str,
    package_id: String,
}

#[derive(Debug, Deserialize)]
struct PackageActivationRequest {
    runtime_version: String,
}

#[derive(Debug, Deserialize)]
struct PackageFailureRequest {
    runtime_version: String,
    stage: String,
    detail: String,
}

#[derive(Debug)]
struct PackageRow {
    package_id: String,
    revision: i64,
    schema_version: i64,
    minimum_runtime_version: String,
    archive_path: String,
    archive_sha256: String,
    archive_size: i64,
    created_at: String,
}

#[derive(Debug)]
struct AuthenticatedDevice {
    device_id: String,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    content_type: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

pub async fn run(config: DeviceApiConfig) -> Result<()> {
    let config = Arc::new(config);
    if let Some(parent) = config.database.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    let addr: SocketAddr = config
        .bind
        .parse()
        .with_context(|| format!("invalid device API bind address {}", config.bind))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind device API at {}", config.bind))?;
    println!("T-Cube device API listening at http://{}", config.bind);

    let app = Router::new()
        .fallback(any(handle_request))
        .with_state(config)
        .layer(TraceLayer::new_for_http());

    axum::serve(listener, app)
        .await
        .context("device API service failed")
}

pub async fn handle_request(
    State(config): State<Arc<DeviceApiConfig>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let request = match HttpRequest::from_request(request).await {
        Ok(request) => request,
        Err(error) => return error_response(400, error.to_string()).into_response(),
    };
    let response =
        match tokio::task::spawn_blocking(move || route_request(&request, config.as_ref())).await {
            Ok(response) => response,
            Err(error) => {
                return error_response(500, format!("device API request failed: {error}"))
                    .into_response()
            }
        };
    response.into_response()
}

fn route_request(request: &HttpRequest, config: &DeviceApiConfig) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/device/v1/content/latest") => match authenticated_device(config, request) {
            Ok(device) => match latest_package(config, &device.device_id) {
                Ok(Some(package)) => {
                    let etag = format!("\"{}\"", package.package_id);
                    if request.header("if-none-match") == Some(etag.as_str()) {
                        HttpResponse::empty(304, vec![("ETag".to_string(), etag)])
                    } else {
                        json_response(
                            200,
                            latest_package_response(package),
                            vec![("ETag".to_string(), etag)],
                        )
                    }
                }
                Ok(None) => error_response(404, "no published content package"),
                Err(error) => error_response(500, error.to_string()),
            },
            Err(error) => error_response(401, error.to_string()),
        },
        ("GET", path) if path.starts_with("/api/device/v1/content/packages/") => {
            match authenticated_device(config, request) {
                Ok(device) => match path.strip_prefix("/api/device/v1/content/packages/") {
                    Some(package_id) => {
                        match downloadable_package(config, &device.device_id, package_id) {
                            Ok(Some(package)) => match fs::read(&package.archive_path) {
                                Ok(bytes) => HttpResponse {
                                    status: 200,
                                    content_type: "application/zip",
                                    headers: vec![
                                        (
                                            "Content-Disposition".to_string(),
                                            format!(
                                                "attachment; filename=\"{}\"",
                                                package_filename(&package.archive_path)
                                            ),
                                        ),
                                        (
                                            "Content-Length".to_string(),
                                            package.archive_size.to_string(),
                                        ),
                                        (
                                            "ETag".to_string(),
                                            format!("\"{}\"", package.archive_sha256),
                                        ),
                                    ],
                                    body: bytes,
                                },
                                Err(error) => error_response(404, error.to_string()),
                            },
                            Ok(None) => error_response(404, "content package not found"),
                            Err(error) => error_response(500, error.to_string()),
                        }
                    }
                    None => error_response(404, "content package not found"),
                },
                Err(error) => error_response(401, error.to_string()),
            }
        }
        ("POST", path)
            if path.starts_with("/api/device/v1/content/packages/")
                && path.ends_with("/activated") =>
        {
            match authenticated_device(config, request) {
                Ok(device) => match path.strip_suffix("/activated") {
                    Some(package_path) => {
                        match package_path.strip_prefix("/api/device/v1/content/packages/") {
                            Some(package_id) => {
                                match parse_json::<PackageActivationRequest>(&request.body) {
                                    Ok(body) => match acknowledge_activation(
                                        config,
                                        &device.device_id,
                                        package_id,
                                        &body.runtime_version,
                                    ) {
                                        Ok(()) => json_response(
                                            200,
                                            PackageAcknowledgementResponse {
                                                status: "activated",
                                                package_id: package_id.to_string(),
                                            },
                                            Vec::new(),
                                        ),
                                        Err(error) => error_response(400, error.to_string()),
                                    },
                                    Err(error) => error_response(400, error.to_string()),
                                }
                            }
                            None => error_response(404, "content package not found"),
                        }
                    }
                    None => error_response(404, "content package not found"),
                },
                Err(error) => error_response(401, error.to_string()),
            }
        }
        ("POST", path)
            if path.starts_with("/api/device/v1/content/packages/")
                && path.ends_with("/failed") =>
        {
            match authenticated_device(config, request) {
                Ok(device) => match path.strip_suffix("/failed") {
                    Some(package_path) => {
                        match package_path.strip_prefix("/api/device/v1/content/packages/") {
                            Some(package_id) => {
                                match parse_json::<PackageFailureRequest>(&request.body) {
                                    Ok(body) => match record_failure(
                                        config,
                                        &device.device_id,
                                        package_id,
                                        &body.runtime_version,
                                        &body.stage,
                                        &body.detail,
                                    ) {
                                        Ok(()) => json_response(
                                            200,
                                            PackageAcknowledgementResponse {
                                                status: "recorded",
                                                package_id: package_id.to_string(),
                                            },
                                            Vec::new(),
                                        ),
                                        Err(error) => error_response(400, error.to_string()),
                                    },
                                    Err(error) => error_response(400, error.to_string()),
                                }
                            }
                            None => error_response(404, "content package not found"),
                        }
                    }
                    None => error_response(404, "content package not found"),
                },
                Err(error) => error_response(401, error.to_string()),
            }
        }
        _ => error_response(405, "method not allowed"),
    }
}

fn latest_package(config: &DeviceApiConfig, device_id: &str) -> Result<Option<PackageRow>> {
    let conn = open_database(config)?;
    let mut statement = conn.prepare(
        "select package_id, revision, schema_version, minimum_runtime_version, archive_path, archive_sha256, archive_size, created_at \
         from content_packages \
         where device_id = ?1 and status in ('published', 'active') \
         order by revision desc \
         limit 1",
    )?;
    statement
        .query_row(params![device_id], package_row)
        .optional()
        .context("failed to read latest device package")
}

fn downloadable_package(
    config: &DeviceApiConfig,
    device_id: &str,
    package_id: &str,
) -> Result<Option<PackageRow>> {
    let conn = open_database(config)?;
    let mut statement = conn.prepare(
        "select package_id, revision, schema_version, minimum_runtime_version, archive_path, archive_sha256, archive_size, created_at \
         from content_packages \
         where device_id = ?1 and package_id = ?2 and status in ('published', 'active')",
    )?;
    statement
        .query_row(params![device_id, package_id], package_row)
        .optional()
        .context("failed to read downloadable device package")
}

fn acknowledge_activation(
    config: &DeviceApiConfig,
    device_id: &str,
    package_id: &str,
    runtime_version: &str,
) -> Result<()> {
    let mut conn = open_database(config)?;
    let transaction = conn.transaction()?;
    let target = transaction
        .prepare(
            "select 1 from content_packages where device_id = ?1 and package_id = ?2 and status in ('published', 'active')",
        )?
        .query_row(params![device_id, package_id], |_| Ok(()))
        .optional()?;
    if target.is_none() {
        return Err(anyhow::anyhow!("published content package not found"));
    }
    transaction.execute(
        "update content_packages set status = 'superseded' where device_id = ?1 and status = 'active' and package_id != ?2",
        params![device_id, package_id],
    )?;
    transaction.execute(
        "update content_packages set status = 'active', activated_at = ?1, activated_runtime_version = ?2 where device_id = ?3 and package_id = ?4",
        params![utc_now(), runtime_version, device_id, package_id],
    )?;
    transaction.commit()?;
    Ok(())
}

fn record_failure(
    config: &DeviceApiConfig,
    device_id: &str,
    package_id: &str,
    runtime_version: &str,
    stage: &str,
    detail: &str,
) -> Result<()> {
    if downloadable_package(config, device_id, package_id)?.is_none() {
        return Err(anyhow::anyhow!("published content package not found"));
    }
    let conn = open_database(config)?;
    conn.execute(
        "insert into content_package_failures \
          (device_id, package_id, runtime_version, stage, detail, occurred_at) \
         values (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            device_id,
            package_id,
            runtime_version,
            stage,
            detail,
            utc_now()
        ],
    )?;
    Ok(())
}

fn authenticated_device(
    config: &DeviceApiConfig,
    request: &HttpRequest,
) -> Result<AuthenticatedDevice> {
    let device_id = request
        .header(DEVICE_ID_HEADER)
        .ok_or_else(|| anyhow::anyhow!("device authentication required"))?
        .trim()
        .to_lowercase();
    let token = request
        .header(DEVICE_TOKEN_HEADER)
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("device authentication required"))?;
    let conn = open_database(config)?;
    let row = conn
        .prepare("select token_hash, revoked_at from devices where id = ?1")?
        .query_row(params![&device_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .optional()?;
    let Some((token_hash, revoked_at)) = row else {
        return Err(anyhow::anyhow!("invalid device credentials"));
    };
    if revoked_at.is_some() || !constant_time_eq(&token_hash, &sha256_hex(token)) {
        return Err(anyhow::anyhow!("invalid device credentials"));
    }
    conn.execute(
        "update devices set last_seen_at = ?1 where id = ?2",
        params![utc_now(), &device_id],
    )?;
    Ok(AuthenticatedDevice { device_id })
}

fn open_database(config: &DeviceApiConfig) -> Result<Connection> {
    if let Some(parent) = config.database.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    migrate_database(&conn)?;
    Ok(conn)
}

fn migrate_database(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        create table if not exists devices (
          id text primary key,
          label text not null,
          token_hash text not null,
          created_at text not null,
          last_seen_at text,
          revoked_at text
        );

        create table if not exists content_packages (
          package_id text primary key,
          device_id text not null,
          revision integer not null,
          schema_version integer not null,
          minimum_runtime_version text not null,
          archive_path text,
          archive_sha256 text,
          archive_size integer,
          status text not null check (status in ('building', 'built', 'published', 'active', 'superseded')),
          created_at text not null,
          published_at text,
          activated_at text,
          activated_runtime_version text,
          foreign key (device_id) references devices(id),
          unique (device_id, revision)
        );

        create table if not exists content_package_failures (
          id integer primary key autoincrement,
          device_id text not null,
          package_id text not null,
          runtime_version text not null,
          stage text not null,
          detail text not null,
          occurred_at text not null,
          foreign key (device_id) references devices(id),
          foreign key (package_id) references content_packages(package_id)
        );
        ",
    )
    .context("failed to initialize device sync SQLite schema")?;
    Ok(())
}

fn package_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PackageRow> {
    Ok(PackageRow {
        package_id: row.get(0)?,
        revision: row.get(1)?,
        schema_version: row.get(2)?,
        minimum_runtime_version: row.get(3)?,
        archive_path: row.get(4)?,
        archive_sha256: row.get(5)?,
        archive_size: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn latest_package_response(package: PackageRow) -> LatestPackageResponse {
    LatestPackageResponse {
        schema_version: package.schema_version,
        revision: package.revision,
        package_id: package.package_id.clone(),
        created_at: package.created_at,
        archive_size: package.archive_size,
        archive_sha256: package.archive_sha256,
        minimum_runtime_version: package.minimum_runtime_version,
        download_url: format!("/api/device/v1/content/packages/{}", package.package_id),
    }
}

fn package_filename(archive_path: &str) -> String {
    Path::new(archive_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("content-package.zip")
        .to_string()
}

fn sha256_hex(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.as_bytes().iter().zip(right.as_bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn utc_now() -> String {
    Utc::now().to_rfc3339()
}

fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).context("invalid request body")
}

fn json_response<T: Serialize>(
    status: u16,
    body: T,
    headers: Vec<(String, String)>,
) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        headers,
        body: serde_json::to_vec(&body).expect("serializing JSON response should not fail"),
    }
}

fn error_response(status: u16, detail: impl Into<String>) -> HttpResponse {
    json_response(status, json!({ "detail": detail.into() }), Vec::new())
}

impl HttpRequest {
    async fn from_request(request: Request<Body>) -> Result<Self> {
        let (parts, body) = request.into_parts();
        let body = to_bytes(body, MAX_REQUEST_BODY_BYTES)
            .await
            .context("failed to read HTTP request body")?;
        let headers = parts
            .headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_ascii_lowercase(), value.trim().to_string()))
            })
            .collect();
        Ok(Self {
            method: parts.method.as_str().to_string(),
            path: parts.uri.path().to_string(),
            headers,
            body: body.to_vec(),
        })
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

impl HttpResponse {
    fn empty(status: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            status,
            content_type: "application/octet-stream",
            headers,
            body: Vec::new(),
        }
    }
}

impl IntoResponse for HttpResponse {
    fn into_response(self) -> Response {
        let body_len = self.body.len().to_string();
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let headers = response.headers_mut();
        insert_header(
            headers,
            HeaderName::from_static("content-type"),
            self.content_type,
        );
        insert_header(
            headers,
            HeaderName::from_static("content-length"),
            &body_len,
        );
        for (name, value) in self.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(name, value);
            }
        }
        response
    }
}

fn insert_header(headers: &mut axum::http::HeaderMap, name: HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::try_from(value) {
        headers.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use tempfile::TempDir;

    #[test]
    fn latest_package_returns_etag_and_download_url() {
        let dir = TempDir::new().unwrap();
        let database = dir.path().join("data/tcube.sqlite3");
        let config = DeviceApiConfig {
            bind: "127.0.0.1:0".to_string(),
            database: database.clone(),
        };
        seed_package(&config, "device-1", "pkg-1", "published");

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/device/v1/content/latest".to_string(),
            headers: HashMap::from([
                (DEVICE_ID_HEADER.to_string(), "device-1".to_string()),
                (DEVICE_TOKEN_HEADER.to_string(), "Bearer secret".to_string()),
            ]),
            body: Vec::new(),
        };

        let response = route_request(&request, &config);
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["package_id"], "pkg-1");
        assert_eq!(response.headers[0].0, "ETag");
    }

    fn seed_package(config: &DeviceApiConfig, device_id: &str, package_id: &str, status: &str) {
        let conn = open_database(config).unwrap();
        conn.execute(
            "insert or ignore into devices (id, label, token_hash, created_at) values (?1, 'cube', ?2, ?3)",
            params![device_id, sha256_hex("secret"), utc_now()],
        )
        .unwrap();
        let package_dir = config
            .database
            .parent()
            .unwrap()
            .join("packages")
            .join(device_id);
        create_dir_all(&package_dir).unwrap();
        let archive_path = package_dir.join("pkg.zip");
        write(&archive_path, b"PK").unwrap();
        conn.execute(
            "insert into content_packages (package_id, device_id, revision, schema_version, minimum_runtime_version, archive_path, archive_sha256, archive_size, status, created_at) values (?1, ?2, 1, 1, '0.1.0', ?3, ?4, 2, ?5, ?6)",
            params![package_id, device_id, archive_path.to_string_lossy(), sha256_hex("PK"), status, utc_now()],
        )
        .unwrap();
    }
}
