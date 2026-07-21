use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::config::AdminConfig;
use crate::db::admin::auth::{
    add_cube_membership, create_session, hash_password, now, random_token, session_expires_at,
    sha256_hex, CubeRole,
};
use crate::db::admin::schema::{migrate_admin_database, table_count};
use crate::server::media::{generated_filename, MAX_AUDIO_BYTES};
use crate::server::routes::auth::session_cookie;
use crate::server::routes::setup::setup_review;
use crate::server::speech::{
    cached_speech_provider_health, probe_speech_provider, speech_http_client_with_ca_cert_path,
    speech_provider_default_base_url, speech_provider_health_url, speech_provider_speech_url,
    speech_provider_voices_url, validate_speech_api_url,
};
use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::json;
use tempfile::TempDir;
use tower::ServiceExt;

const MAX_RESPONSE_BODY_BYTES: usize = 25 * 1024 * 1024;

struct TestRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct TestResponse {
    status: u16,
    content_type: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn axum_request(request: &TestRequest, config: &AdminConfig) -> TestResponse {
    let config = Arc::new(AdminConfig {
        bind: config.bind.clone(),
        database: config.database.clone(),
        ui_dist: config.ui_dist.clone(),
        media_root: config.media_root.clone(),
        content_root: config.content_root.clone(),
        hostname: config.hostname.clone(),
        usb_address: config.usb_address.clone(),
        usb_connected: config.usb_connected,
    });
    let app = crate::server::routes::router().with_state(config);
    let uri = request_uri(request);
    let mut builder = Request::builder().method(request.method.as_str()).uri(uri);
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let axum_request = builder
        .body(Body::from(request.body.clone()))
        .expect("test request should be valid");

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build")
        .block_on(async move {
            let response = app
                .oneshot(axum_request)
                .await
                .expect("router should produce a response");
            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_string();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (header_display_name(name.as_str()), value.to_string()))
                })
                .collect();
            let body = to_bytes(response.into_body(), MAX_RESPONSE_BODY_BYTES)
                .await
                .expect("response body should be readable")
                .to_vec();
            TestResponse {
                status,
                content_type,
                headers,
                body,
            }
        })
}

fn request_uri(request: &TestRequest) -> String {
    if request.query.is_empty() {
        return request.path.clone();
    }
    let query = request
        .query
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{}?{}", request.path, query)
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            byte => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn header_display_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("set-cookie") {
        "Set-Cookie".to_string()
    } else {
        name.to_string()
    }
}

#[test]
fn serves_bundled_content_audio_from_content_root() {
    let database = test_database();
    let config = test_config(database.path());
    let audio_path = config.content_root.join("audio/animals/cow-moo.wav");
    fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
    fs::write(&audio_path, b"RIFFtest").unwrap();

    let response = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/content/audio/animals/cow-moo.wav".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        },
        &config,
    );

    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "audio/wav");
    assert_eq!(response.body, b"RIFFtest");
}

#[test]
fn rejects_bundled_content_path_traversal() {
    let database = test_database();
    let config = test_config(database.path());

    let response = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/content/../data/tcube.sqlite3".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        },
        &config,
    );

    assert_eq!(response.status, 400);
}

#[test]
fn returns_default_setup_review_without_database() {
    let config = AdminConfig {
        bind: "127.0.0.1:0".to_string(),
        database: PathBuf::from("/tmp/does-not-exist-tcube.sqlite3"),
        ui_dist: PathBuf::from("admin-ui"),
        media_root: PathBuf::from("data/media"),
        content_root: PathBuf::from("content"),
        hostname: "tcube-a7f3.local".to_string(),
        usb_address: "10.55.0.1".to_string(),
        usb_connected: true,
    };

    let review = setup_review(&config).unwrap();

    assert_eq!(review.cube_name, "T-Cube");
    assert_eq!(review.dashboard_address, "https://tcube-a7f3.local/");
    assert!(review.wifi_ssid.is_none());
    assert_eq!(review.button_modes["1"], "language:English");
}

#[test]
fn bootstrap_owner_creates_clean_database_session_and_first_cube_membership() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join("fresh/data/tcube.sqlite3");
    let config = test_config(&database);

    let initial_session = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        },
        &config,
    );
    let initial_body: serde_json::Value = serde_json::from_slice(&initial_session.body).unwrap();
    assert_eq!(initial_session.status, 200);
    assert_eq!(initial_body["authenticated"], false);
    assert_eq!(initial_body["bootstrap_required"], true);

    let bootstrap = axum_request(
        &json_request(
            "POST",
            "/api/auth/bootstrap",
            json!({
                "username": "Parent",
                "display_name": "Parent Admin",
                "password": "owner-password"
            }),
            None,
        ),
        &config,
    );

    assert_eq!(bootstrap.status, 200);
    assert!(database.exists());
    let bootstrap_body: serde_json::Value = serde_json::from_slice(&bootstrap.body).unwrap();
    assert_eq!(bootstrap_body["authenticated"], true);
    assert_eq!(bootstrap_body["bootstrap_required"], false);
    assert_eq!(bootstrap_body["account"]["username"], "parent");
    assert_eq!(bootstrap_body["account"]["display_name"], "Parent Admin");
    assert_eq!(bootstrap_body["cubes"][0]["label"], "T-Cube");
    assert_eq!(bootstrap_body["cubes"][0]["role"], "owner");
    assert!(
        bootstrap_body["cubes"][0]["device_id"]
            .as_str()
            .unwrap()
            .len()
            > 10
    );
    let cookie = bootstrap
        .headers
        .iter()
        .find(|(name, _)| name == "Set-Cookie")
        .map(|(_, value)| value.clone())
        .unwrap();
    assert!(cookie.starts_with("tcube_session="));

    let name_response = axum_request(
        &json_request(
            "POST",
            "/api/setup/name",
            json!({ "cube_name": "Nursery Cube" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(name_response.status, 200);
    let name_body: serde_json::Value = serde_json::from_slice(&name_response.body).unwrap();
    let device_id = name_body["device_id"].as_str().unwrap();
    assert_eq!(
        Some(device_id),
        bootstrap_body["cubes"][0]["device_id"].as_str()
    );

    let session_response = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    let session_body: serde_json::Value = serde_json::from_slice(&session_response.body).unwrap();
    assert_eq!(session_response.status, 200);
    assert_eq!(session_body["authenticated"], true);
    assert_eq!(session_body["cubes"][0]["device_id"], device_id);
    assert_eq!(session_body["cubes"][0]["role"], "owner");

    let review = setup_review(&config).unwrap();
    assert_eq!(review.cube_name, "Nursery Cube");
    assert_eq!(review.button_modes["1"], "language:English");
    assert_eq!(review.active_counts["language"], 10);
    assert_eq!(review.active_counts["animals"], 10);
    assert_eq!(review.active_counts["music"], 10);

    let wifi_response = axum_request(
        &json_request(
            "POST",
            "/api/setup/wifi/verified",
            json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(wifi_response.status, 200);

    let complete_response = axum_request(
        &TestRequest {
            method: "POST".to_string(),
            path: "/api/setup/complete".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(complete_response.status, 200);
    let complete_body: serde_json::Value = serde_json::from_slice(&complete_response.body).unwrap();
    assert_eq!(complete_body["status"], "complete");
}

#[test]
fn password_login_sets_session_cookie_and_authenticates_session() {
    let database = test_database();
    let config = test_config(database.path());
    seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let request = json_request(
        "POST",
        "/api/auth/login/password",
        json!({ "username": "admin", "password": "secret-password" }),
        None,
    );

    let response = axum_request(&request, &config);

    assert_eq!(response.status, 200);
    let cookie = response
        .headers
        .iter()
        .find(|(name, _)| name == "Set-Cookie")
        .map(|(_, value)| value.clone())
        .unwrap();
    assert!(cookie.starts_with("tcube_session="));

    let session_request = TestRequest {
        method: "GET".to_string(),
        path: "/api/auth/session".to_string(),
        query: HashMap::new(),
        headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
        body: Vec::new(),
    };
    let session_response = axum_request(&session_request, &config);
    let body: serde_json::Value = serde_json::from_slice(&session_response.body).unwrap();

    assert_eq!(session_response.status, 200);
    assert_eq!(body["authenticated"], true);
    assert_eq!(body["account"]["username"], "admin");

    let events_response = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/pi/v1/events/recent".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(events_response.status, 200);
    let events_body: serde_json::Value = serde_json::from_slice(&events_response.body).unwrap();
    assert_eq!(events_body[0]["kind"], "signed_in");
    assert_eq!(events_body[0]["text"], "admin");
}

#[test]
fn single_account_without_membership_is_repaired_as_owner_on_session_read() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "rms", "secret-password").unwrap();
    let conn = Connection::open(database.path()).unwrap();
    conn.execute("delete from cube_memberships", []).unwrap();
    conn.execute("delete from devices", []).unwrap();
    conn.execute("update device_setup set device_id = null where id = 1", [])
        .unwrap();
    let token = create_session(&conn, &account_id).unwrap();
    drop(conn);

    let session = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
            body: Vec::new(),
        },
        &config,
    );

    assert_eq!(session.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&session.body).unwrap();
    assert_eq!(body["authenticated"], true);
    assert_eq!(body["account"]["username"], "rms");
    assert_eq!(body["cubes"][0]["role"], "owner");
    assert!(body["cubes"][0]["device_id"].as_str().unwrap().len() > 10);
    let repaired_count: i64 = Connection::open(database.path())
        .unwrap()
        .query_row("select count(*) from cube_memberships", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(repaired_count, 1);
}

#[test]
fn versioned_admin_api_aliases_support_session_setup_and_events() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let conn = Connection::open(database.path()).unwrap();
    let cookie = session_cookie(&create_session(&conn, &account_id).unwrap());
    conn.execute_batch(
        "create table button_events (
            id integer primary key autoincrement,
            occurred_at text not null,
            button_id integer not null,
            mode text not null,
            response_id text not null,
            response_text text not null
        );",
    )
    .unwrap();
    for index in 0..12 {
        conn.execute(
            "insert into button_events \
             (occurred_at, button_id, mode, response_id, response_text) \
             values (?1, 1, 'language', ?2, ?3)",
            params![
                format!("2026-07-01T00:{index:02}:00Z"),
                format!("hello-{index}"),
                format!("Hello {index}")
            ],
        )
        .unwrap();
    }
    drop(conn);

    let session = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/pi/v1/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(session.status, 200);

    let mode = axum_request(
        &json_request(
            "POST",
            "/api/pi/v1/setup/buttons/1/mode",
            json!({ "mode": "language", "language": "French" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(mode.status, 200);

    let events = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/pi/v1/events/recent".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(events.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&events.body).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 10);
    assert_eq!(body[0]["kind"], "button_pressed");
    assert_eq!(body[0]["button_id"], 1);
    assert_eq!(body[0]["button_label"], "Top");
    assert_eq!(body[0]["response_text"], "Hello 11");
    assert_eq!(body[9]["response_text"], "Hello 2");
}

#[test]
fn invalid_password_fails() {
    let database = test_database();
    let config = test_config(database.path());
    seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let request = json_request(
        "POST",
        "/api/auth/login/password",
        json!({ "username": "admin", "password": "wrong-password" }),
        None,
    );

    let response = axum_request(&request, &config);

    assert_eq!(response.status, 400);
}

#[test]
fn recovery_code_resets_password_and_revokes_sessions() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "old-password").unwrap();
    let conn = Connection::open(database.path()).unwrap();
    let old_token = create_session(&conn, &account_id).unwrap();
    let recovery_code = "recovery-code";
    conn.execute(
        "insert into recovery_codes (id, account_id, code_hash, created_at, expires_at) \
         values ('recovery-1', ?1, ?2, ?3, ?4)",
        params![
            account_id,
            sha256_hex(recovery_code),
            now(),
            session_expires_at()
        ],
    )
    .unwrap();
    drop(conn);

    let recover = json_request(
        "POST",
        "/api/auth/recover",
        json!({ "code": recovery_code, "password": "new-password" }),
        None,
    );
    let recover_response = axum_request(&recover, &config);
    assert_eq!(recover_response.status, 200);

    let old_session = TestRequest {
        method: "GET".to_string(),
        path: "/api/auth/session".to_string(),
        query: HashMap::new(),
        headers: HashMap::from([("cookie".to_string(), format!("tcube_session={old_token}"))]),
        body: Vec::new(),
    };
    let old_response = axum_request(&old_session, &config);
    let old_body: serde_json::Value = serde_json::from_slice(&old_response.body).unwrap();
    assert_eq!(old_body["authenticated"], false);

    let login = json_request(
        "POST",
        "/api/auth/login/password",
        json!({ "username": "admin", "password": "new-password" }),
        None,
    );
    assert_eq!(axum_request(&login, &config).status, 200);
}

#[test]
fn authenticated_user_can_create_and_use_recovery_code() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "old-password").unwrap();
    let conn = Connection::open(database.path()).unwrap();
    let token = create_session(&conn, &account_id).unwrap();
    drop(conn);
    let cookie = session_cookie(&token);

    let create_response = axum_request(
        &TestRequest {
            method: "POST".to_string(),
            path: "/api/auth/recovery-code".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(create_response.status, 200);
    let create_body: serde_json::Value = serde_json::from_slice(&create_response.body).unwrap();
    let code = create_body["code"].as_str().unwrap();
    assert!(!code.is_empty());
    assert!(create_body["expires_at"].as_str().unwrap().ends_with('Z'));

    let recover_response = axum_request(
        &json_request(
            "POST",
            "/api/auth/recover",
            json!({ "code": code, "password": "new-password" }),
            None,
        ),
        &config,
    );
    assert_eq!(recover_response.status, 200);

    let old_session = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
            body: Vec::new(),
        },
        &config,
    );
    let old_body: serde_json::Value = serde_json::from_slice(&old_session.body).unwrap();
    assert_eq!(old_body["authenticated"], false);

    let login = json_request(
        "POST",
        "/api/auth/login/password",
        json!({ "username": "admin", "password": "new-password" }),
        None,
    );
    assert_eq!(axum_request(&login, &config).status, 200);
}

#[test]
fn manager_invitation_can_be_created_and_accepted_once() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );

    let invitation = axum_request(
        &json_request(
            "POST",
            "/api/auth/invitations",
            json!({ "device_id": "device-1" }),
            Some(cookie),
        ),
        &config,
    );
    assert_eq!(invitation.status, 200);
    let invitation_body: serde_json::Value = serde_json::from_slice(&invitation.body).unwrap();
    assert_eq!(invitation_body["role"], "manager");
    assert_eq!(invitation_body["device_id"], "device-1");
    let code = invitation_body["code"].as_str().unwrap();

    let accepted = axum_request(
        &json_request(
            "POST",
            "/api/auth/invitations/accept",
            json!({
                "code": code,
                "username": "manager",
                "display_name": "Manager",
                "password": "manager-password"
            }),
            None,
        ),
        &config,
    );
    assert_eq!(accepted.status, 200);
    assert!(accepted
        .headers
        .iter()
        .any(|(name, value)| name == "Set-Cookie" && value.starts_with("tcube_session=")));
    let accepted_body: serde_json::Value = serde_json::from_slice(&accepted.body).unwrap();
    assert_eq!(accepted_body["authenticated"], true);
    assert_eq!(accepted_body["account"]["username"], "manager");
    assert_eq!(accepted_body["cubes"][0]["device_id"], "device-1");
    assert_eq!(accepted_body["cubes"][0]["role"], "manager");

    let second_accept = axum_request(
        &json_request(
            "POST",
            "/api/auth/invitations/accept",
            json!({
                "code": code,
                "username": "manager-two",
                "display_name": "Manager Two",
                "password": "manager-password"
            }),
            None,
        ),
        &config,
    );
    assert_eq!(second_accept.status, 400);
}

#[test]
fn logout_revokes_session() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let conn = Connection::open(database.path()).unwrap();
    let token = create_session(&conn, &account_id).unwrap();
    drop(conn);
    let logout_request = TestRequest {
        method: "POST".to_string(),
        path: "/api/auth/logout".to_string(),
        query: HashMap::new(),
        headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
        body: Vec::new(),
    };

    let logout_response = axum_request(&logout_request, &config);

    assert_eq!(logout_response.status, 200);
    let session_request = TestRequest {
        method: "GET".to_string(),
        path: "/api/auth/session".to_string(),
        query: HashMap::new(),
        headers: HashMap::from([("cookie".to_string(), format!("tcube_session={token}"))]),
        body: Vec::new(),
    };
    let session_response = axum_request(&session_request, &config);
    let body: serde_json::Value = serde_json::from_slice(&session_response.body).unwrap();
    assert_eq!(body["authenticated"], false);
}

#[test]
fn setup_name_and_wifi_mutations_persist() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );

    let name_response = axum_request(
        &json_request(
            "POST",
            "/api/setup/name",
            json!({ "cube_name": "Nursery Cube" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(name_response.status, 200);
    let name_body: serde_json::Value = serde_json::from_slice(&name_response.body).unwrap();
    assert_eq!(name_body["name"], "Nursery Cube");
    assert_eq!(name_body["token"], serde_json::Value::Null);

    let wifi_response = axum_request(
        &json_request(
            "POST",
            "/api/setup/wifi/verified",
            json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
            Some(cookie),
        ),
        &config,
    );
    assert_eq!(wifi_response.status, 200);

    let review = setup_review(&config).unwrap();
    assert_eq!(review.cube_name, "Nursery Cube");
    assert!(review.wifi_verified);
    assert_eq!(review.wifi_ssid.as_deref(), Some("Home WiFi"));
    assert_eq!(review.dashboard_ip.as_deref(), Some("192.168.50.20"));
}

#[test]
fn button_mode_updates_allow_reused_modes_and_languages() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );

    let response = axum_request(
        &json_request(
            "POST",
            "/api/setup/buttons/1/mode",
            json!({ "mode": "language", "language": "Spanish" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        setup_review(&config).unwrap().button_modes["1"],
        "language:Spanish"
    );

    let duplicate = axum_request(
        &json_request(
            "POST",
            "/api/setup/buttons/2/mode",
            json!({ "mode": "language", "language": "Spanish" }),
            Some(cookie),
        ),
        &config,
    );
    assert_eq!(duplicate.status, 200);
    let review = setup_review(&config).unwrap();
    assert_eq!(review.button_modes["1"], "language:Spanish");
    assert_eq!(review.button_modes["2"], "language:Spanish");
}

#[test]
fn manager_can_manage_content_but_not_owner_sensitive_actions() {
    let database = test_database();
    let config = test_config(database.path());
    seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let manager_id = seed_manager_account(database.path()).unwrap();
    let conn = Connection::open(database.path()).unwrap();
    let manager_cookie = session_cookie(&create_session(&conn, &manager_id).unwrap());
    seed_active_content(&conn).unwrap();
    conn.execute(
        "insert into content_items \
         (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
         values ('manager-draft', 'animals', 2, null, 'Roar', 'Roar', 'data/media/recorded/animals/roar.wav', 'recorded', 'archived', 11)",
        [],
    )
    .unwrap();
    drop(conn);

    let list = axum_request(
        &authed_get(
            "/api/content/buttons/2/animals/inactive",
            HashMap::new(),
            &manager_cookie,
        ),
        &config,
    );
    assert_eq!(list.status, 200);

    let mode = axum_request(
        &json_request(
            "POST",
            "/api/setup/buttons/4/mode",
            json!({ "mode": "animals" }),
            Some(manager_cookie.clone()),
        ),
        &config,
    );
    assert_eq!(mode.status, 200);

    let activate = axum_request(
        &TestRequest {
            method: "POST".to_string(),
            path: "/api/content/items/manager-draft/activate".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), manager_cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(activate.status, 200);

    let rename = axum_request(
        &json_request(
            "POST",
            "/api/setup/name",
            json!({ "cube_name": "Manager Rename" }),
            Some(manager_cookie.clone()),
        ),
        &config,
    );
    assert_eq!(rename.status, 400);

    let wifi = axum_request(
        &json_request(
            "POST",
            "/api/setup/wifi/verified",
            json!({ "ssid": "Home WiFi", "dashboard_ip": "192.168.50.20" }),
            Some(manager_cookie.clone()),
        ),
        &config,
    );
    assert_eq!(wifi.status, 400);

    let invitation = axum_request(
        &json_request(
            "POST",
            "/api/auth/invitations",
            json!({ "device_id": "device-1" }),
            Some(manager_cookie.clone()),
        ),
        &config,
    );
    assert_eq!(invitation.status, 400);

    let reset = axum_request(
        &json_request(
            "POST",
            "/api/setup/factory-reset",
            json!({ "confirmation": "FACTORY RESET" }),
            Some(manager_cookie),
        ),
        &config,
    );
    assert_eq!(reset.status, 400);
}

#[test]
fn complete_setup_marks_setup_complete_after_prerequisites() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );
    let conn = Connection::open(database.path()).unwrap();
    conn.execute(
        "update device_setup set cube_name = 'Nursery Cube', wifi_verified_at = ?1 where id = 1",
        [now()],
    )
    .unwrap();
    seed_active_content(&conn).unwrap();
    drop(conn);

    let response = axum_request(
        &TestRequest {
            method: "POST".to_string(),
            path: "/api/setup/complete".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );

    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["status"], "complete");
    let complete: i64 = Connection::open(database.path())
        .unwrap()
        .query_row(
            "select setup_complete from device_setup where id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(complete, 1);
}

#[test]
fn content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );
    let conn = Connection::open(database.path()).unwrap();
    seed_active_content(&conn).unwrap();
    conn.execute_batch(
        "create table if not exists button_events (
            id integer primary key autoincrement,
            occurred_at text not null,
            button_id integer not null,
            mode text not null,
            response_id text not null,
            response_text text not null
        );",
    )
    .unwrap();
    conn.execute(
        "insert into button_events \
         (occurred_at, button_id, mode, response_id, response_text) \
         values (?1, 1, 'language', 'language-one', 'Hello')",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into button_events \
         (occurred_at, button_id, mode, response_id, response_text) \
         values (?1, 2, 'animals', 'language-one', 'Hello')",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into button_events \
         (occurred_at, button_id, mode, response_id, response_text) \
         values (?1, 1, 'language', 'other-response', 'Other')",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into content_items \
         (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
         values \
         ('generated-draft', 'language', 1, 'English', 'Draft', 'Draft', 'data/audio/draft/language/generated.wav', 'generated', 'archived', 10), \
         ('uploaded-language-draft', 'language', 1, 'English', 'Upload', 'Upload', 'data/audio/draft/language/upload.wav', 'uploaded', 'archived', 11), \
         ('recorded-draft', 'animals', 2, null, 'Roar', 'Roar', 'data/audio/draft/animals/roar.wav', 'recorded', 'archived', 12), \
         ('rejected-draft', 'animals', 2, null, 'Growl', 'Growl', 'data/audio/draft/animals/growl.wav', 'recorded', 'archived', 13)",
        [],
    )
    .unwrap();
    let draft_audio = config.media_root.join("draft/animals/roar.wav");
    fs::create_dir_all(draft_audio.parent().unwrap()).unwrap();
    fs::write(&draft_audio, test_wav()).unwrap();
    let rejected_draft_audio = config.media_root.join("draft/animals/growl.wav");
    fs::write(&rejected_draft_audio, test_wav()).unwrap();
    let generated_draft_audio = config.media_root.join("draft/language/generated.wav");
    fs::create_dir_all(generated_draft_audio.parent().unwrap()).unwrap();
    fs::write(&generated_draft_audio, test_wav()).unwrap();
    let uploaded_language_draft_audio = config.media_root.join("draft/language/upload.wav");
    fs::write(&uploaded_language_draft_audio, test_wav()).unwrap();
    drop(conn);

    let active = axum_request(
        &authed_get(
            "/api/content/buttons/1/language/active",
            HashMap::from([("language".to_string(), "English".to_string())]),
            &cookie,
        ),
        &config,
    );
    assert_eq!(active.status, 200);
    let active_body: serde_json::Value = serde_json::from_slice(&active.body).unwrap();
    assert_eq!(active_body["items"].as_array().unwrap().len(), 1);
    assert_eq!(active_body["items"][0]["play_count"], 2);
    assert_eq!(
        active_body["items"][0]["preview_url"],
        serde_json::Value::Null
    );
    assert_eq!(active_body["empty_state"], serde_json::Value::Null);

    let active_animals = axum_request(
        &authed_get(
            "/api/content/buttons/2/animals/active",
            HashMap::new(),
            &cookie,
        ),
        &config,
    );
    assert_eq!(active_animals.status, 200);
    let active_animals_body: serde_json::Value =
        serde_json::from_slice(&active_animals.body).unwrap();
    assert_eq!(active_animals_body["items"][0]["id"], "animal-one");
    assert_eq!(active_animals_body["items"][0]["play_count"], 0);

    let active_language_mismatch = axum_request(
        &authed_get(
            "/api/content/buttons/1/language/active",
            HashMap::from([("language".to_string(), "French".to_string())]),
            &cookie,
        ),
        &config,
    );
    assert_eq!(active_language_mismatch.status, 200);
    let active_language_mismatch_body: serde_json::Value =
        serde_json::from_slice(&active_language_mismatch.body).unwrap();
    assert_eq!(
        active_language_mismatch_body["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        active_language_mismatch_body["empty_state"]["title"],
        "No active French content on this button"
    );
    assert!(active_language_mismatch_body["empty_state"]["detail"]
        .as_str()
        .unwrap()
        .contains("This button has active content in English"));

    let inactive = axum_request(
        &authed_get(
            "/api/content/buttons/2/animals/inactive",
            HashMap::new(),
            &cookie,
        ),
        &config,
    );
    assert_eq!(inactive.status, 200);
    let inactive_body: serde_json::Value = serde_json::from_slice(&inactive.body).unwrap();
    assert_eq!(inactive_body["items"][0]["id"], "recorded-draft");
    assert_eq!(
        inactive_body["items"][0]["preview_url"],
        "/api/media/draft/animals/roar.wav"
    );
    assert_eq!(inactive_body["empty_state"], serde_json::Value::Null);

    let inactive_language_mismatch = axum_request(
        &authed_get(
            "/api/content/buttons/1/language/inactive",
            HashMap::from([("language".to_string(), "French".to_string())]),
            &cookie,
        ),
        &config,
    );
    assert_eq!(inactive_language_mismatch.status, 200);
    let inactive_language_mismatch_body: serde_json::Value =
        serde_json::from_slice(&inactive_language_mismatch.body).unwrap();
    assert_eq!(
        inactive_language_mismatch_body["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        inactive_language_mismatch_body["empty_state"]["title"],
        "No draft French content on this button"
    );
    assert!(inactive_language_mismatch_body["empty_state"]["detail"]
        .as_str()
        .unwrap()
        .contains("This button has draft content in English"));

    let activate = axum_request(
        &TestRequest {
            method: "POST".to_string(),
            path: "/api/content/items/recorded-draft/activate".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(activate.status, 200);
    let (activated_state, activated_audio_path): (String, String) =
        Connection::open(database.path())
            .unwrap()
            .query_row(
                "select state, audio_path from content_items where id = 'recorded-draft'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
    assert_eq!(activated_state, "active");
    assert_eq!(activated_audio_path, "data/audio/active/animals/roar.wav");
    assert!(!draft_audio.exists());
    assert!(config.media_root.join("active/animals/roar.wav").exists());

    let cleanup = axum_request(
        &json_request(
            "DELETE",
            "/api/content/generated-speech/unused",
            json!({ "button_id": 1, "language": "English" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(cleanup.status, 200);
    let cleanup_body: serde_json::Value = serde_json::from_slice(&cleanup.body).unwrap();
    assert_eq!(cleanup_body["deleted_count"], 2);
    assert!(!generated_draft_audio.exists());
    assert!(!uploaded_language_draft_audio.exists());

    let reject = axum_request(
        &TestRequest {
            method: "DELETE".to_string(),
            path: "/api/content/items/rejected-draft".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(reject.status, 200);
    assert!(!rejected_draft_audio.exists());

    let trash = axum_request(
        &TestRequest {
            method: "DELETE".to_string(),
            path: "/api/content/items/language-one".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(trash.status, 200);
    let trashed_state: String = Connection::open(database.path())
        .unwrap()
        .query_row(
            "select state from content_items where id = 'language-one'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trashed_state, "trash");

    let events = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/pi/v1/events/recent".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(events.status, 200);
    let events_body: serde_json::Value = serde_json::from_slice(&events.body).unwrap();
    let kinds = events_body
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"content_activated"));
    assert!(kinds.contains(&"content_deleted"));
    assert!(events_body.as_array().unwrap().iter().any(|event| {
        event["kind"] == "content_activated" && event["audio_filename"] == "roar.wav"
    }));
}

#[test]
fn content_inventory_classifies_current_drafts_and_unused_audio() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );
    let conn = Connection::open(database.path()).unwrap();
    seed_active_content(&conn).unwrap();
    conn.execute(
        "insert into content_items \
         (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
         values \
         ('inventory-active-one', 'language', 1, 'English', 'Hello audio', 'Hello audio', 'data/audio/active/language/hello.wav', 'recorded', 'active', 20), \
         ('inventory-active-two', 'language', 1, 'English', 'Bye audio', 'Bye audio', 'data/audio/active/language/bye.wav', 'recorded', 'active', 21), \
         ('inventory-draft', 'language', 1, 'French', 'Bonjour', 'Bonjour', 'data/audio/draft/language/bonjour.wav', 'recorded', 'archived', 22)",
        [],
    )
    .unwrap();
    drop(conn);
    fs::create_dir_all(config.media_root.join("active/language")).unwrap();
    let hello_audio = config.media_root.join("active/language/hello.wav");
    let bye_audio = config.media_root.join("active/language/bye.wav");
    fs::write(&hello_audio, b"hello").unwrap();
    fs::write(&bye_audio, b"bye").unwrap();

    let mode = axum_request(
        &json_request(
            "POST",
            "/api/setup/buttons/1/mode",
            json!({ "mode": "language", "language": "French" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(mode.status, 200);

    let inventory = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/content/inventory".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(inventory.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&inventory.body).unwrap();
    assert_eq!(body["draft_count"], 1);
    assert_eq!(body["unused_count"], 2);
    assert_eq!(body["active_count"], 0);
    let draft = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "inventory-draft")
        .unwrap();
    assert_eq!(draft["status"], "draft");
    let unused = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "inventory-active-one")
        .unwrap();
    assert_eq!(unused["status"], "unused");
    assert!(unused["reason"]
        .as_str()
        .unwrap()
        .contains("Button 1 is set to French"));

    let cleanup = axum_request(
        &TestRequest {
            method: "DELETE".to_string(),
            path: "/api/content/unused".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(cleanup.status, 200);
    let cleanup_body: serde_json::Value = serde_json::from_slice(&cleanup.body).unwrap();
    assert_eq!(cleanup_body["deleted_count"], 2);
    assert!(!hello_audio.exists());
    assert!(!bye_audio.exists());
    let trashed_count: i64 = Connection::open(database.path())
        .unwrap()
        .query_row(
            "select count(*) from content_items where id in ('inventory-active-one', 'inventory-active-two') and state = 'trash'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trashed_count, 2);
}

#[test]
fn factory_reset_requires_confirmation_and_restores_first_run_defaults() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let token = create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap();
    let cookie = session_cookie(&token);

    let wrong_confirmation = axum_request(
        &json_request(
            "POST",
            "/api/pi/v1/setup/factory-reset",
            json!({ "confirmation": "reset" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(wrong_confirmation.status, 400);

    let session_before = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie.clone())]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(session_before.status, 200);

    let conn = Connection::open(database.path()).unwrap();
    migrate_admin_database(&conn, &config).unwrap();
    conn.execute(
        "update device_setup \
         set setup_complete = 1, cube_name = 'Nursery Cube', wifi_ssid = 'Home', \
             wifi_verified_at = ?1, dashboard_ip = '192.168.50.20', updated_at = ?1 \
         where id = 1",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into trusted_sessions (id, label, created_at) values ('trusted-1', 'Phone', ?1)",
        [now()],
    )
    .unwrap();
    conn.execute(
        "update audio_settings set volume_percent = 85, updated_at = ?1 where id = 1",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into content_items \
         (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
         values \
         ('reset-active', 'language', 1, 'English', 'Parent active', 'Parent active', 'data/audio/active/language/parent.wav', 'recorded', 'active', 50), \
         ('reset-draft', 'language', 1, 'English', 'Parent draft', 'Parent draft', 'data/audio/draft/language/draft.wav', 'uploaded', 'archived', 51)",
        [],
    )
    .unwrap();
    conn.execute(
        "insert into media_artifacts (id, content_item_id, media_type, path, state) \
         values ('artifact-1', 'reset-active', 'recorded_audio', 'data/audio/active/language/parent.wav', 'active')",
        [],
    )
    .unwrap();
    conn.execute(
        "insert into content_jobs (id, job_type, status, language) \
         values ('job-1', 'tts', 'queued', 'English')",
        [],
    )
    .unwrap();
    conn.execute(
        "insert into button_events (occurred_at, button_id, mode, response_id, response_text) \
         values (?1, 1, 'language', 'reset-active', 'Parent active')",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into setup_debug_events (event_type, button_id, details) \
         values ('setup_help_button_press', 4, '{}')",
        [],
    )
    .unwrap();
    conn.execute(
        "insert into content_packages \
         (package_id, device_id, revision, schema_version, minimum_runtime_version, status, created_at) \
         values ('package-1', 'device-1', 1, 1, '0.0.1', 'active', ?1)",
        [now()],
    )
    .unwrap();
    conn.execute(
        "insert into content_package_failures \
         (device_id, package_id, runtime_version, stage, detail, occurred_at) \
         values ('device-1', 'package-1', '0.0.1', 'download', 'failed', ?1)",
        [now()],
    )
    .unwrap();
    drop(conn);

    let active_audio = config.media_root.join("active/language/parent.wav");
    let draft_audio = config.media_root.join("draft/language/draft.wav");
    fs::create_dir_all(active_audio.parent().unwrap()).unwrap();
    fs::create_dir_all(draft_audio.parent().unwrap()).unwrap();
    fs::write(&active_audio, b"active").unwrap();
    fs::write(&draft_audio, b"draft").unwrap();

    let reset = axum_request(
        &json_request(
            "POST",
            "/api/pi/v1/setup/factory-reset",
            json!({ "confirmation": "FACTORY RESET" }),
            Some(cookie.clone()),
        ),
        &config,
    );
    assert_eq!(reset.status, 200);
    assert!(reset
        .headers
        .iter()
        .any(|(name, value)| name == "Set-Cookie" && value.starts_with("tcube_session=;")));
    let reset_body: serde_json::Value = serde_json::from_slice(&reset.body).unwrap();
    assert_eq!(reset_body["bootstrap_required"], true);
    assert!(!active_audio.exists());
    assert!(!draft_audio.exists());

    let conn = Connection::open(database.path()).unwrap();
    assert_eq!(table_count(&conn, "admin_accounts").unwrap(), 0);
    assert_eq!(table_count(&conn, "admin_sessions").unwrap(), 0);
    assert_eq!(table_count(&conn, "button_events").unwrap(), 0);
    assert_eq!(table_count(&conn, "setup_debug_events").unwrap(), 0);
    assert_eq!(table_count(&conn, "content_jobs").unwrap(), 0);
    assert_eq!(table_count(&conn, "media_artifacts").unwrap(), 0);
    assert_eq!(table_count(&conn, "content_packages").unwrap(), 0);
    assert_eq!(table_count(&conn, "content_package_failures").unwrap(), 0);
    assert_eq!(table_count(&conn, "button_mappings").unwrap(), 5);
    assert_eq!(table_count(&conn, "content_items").unwrap(), 30);
    let setup_complete: i64 = conn
        .query_row(
            "select setup_complete from device_setup where id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(setup_complete, 0);
    let volume_percent: i64 = conn
        .query_row(
            "select volume_percent from audio_settings where id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(volume_percent, 50);
    let top_mode: String = conn
        .query_row(
            "select mode || ':' || coalesce(language, '') from button_mappings where button_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(top_mode, "language:English");
    drop(conn);

    let session_after = axum_request(
        &TestRequest {
            method: "GET".to_string(),
            path: "/api/auth/session".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("cookie".to_string(), cookie)]),
            body: Vec::new(),
        },
        &config,
    );
    assert_eq!(session_after.status, 200);
    let session_after_body: serde_json::Value =
        serde_json::from_slice(&session_after.body).unwrap();
    assert_eq!(session_after_body["authenticated"], false);
    assert_eq!(session_after_body["bootstrap_required"], true);

    let rebootstrap = axum_request(
        &json_request(
            "POST",
            "/api/auth/bootstrap",
            json!({
                "username": "rms",
                "display_name": "rms",
                "password": "owner-password"
            }),
            None,
        ),
        &config,
    );
    assert_eq!(rebootstrap.status, 200);
    let rebootstrap_body: serde_json::Value = serde_json::from_slice(&rebootstrap.body).unwrap();
    assert_eq!(rebootstrap_body["authenticated"], true);
    assert_eq!(rebootstrap_body["account"]["username"], "rms");
    assert_eq!(rebootstrap_body["cubes"][0]["role"], "owner");
    assert!(
        rebootstrap_body["cubes"][0]["device_id"]
            .as_str()
            .unwrap()
            .len()
            > 10
    );
}

#[test]
fn multipart_recording_and_upload_create_inactive_drafts() {
    let database = test_database();
    let config = test_config(database.path());
    let account_id = seed_auth_database(database.path(), "admin", "secret-password").unwrap();
    let cookie = session_cookie(
        &create_session(&Connection::open(database.path()).unwrap(), &account_id).unwrap(),
    );
    let wav = test_wav();

    let recorded = axum_request(
        &multipart_request(
            "/api/content/recordings",
            &cookie,
            vec![
                ("content_type", "language"),
                ("button_id", "1"),
                ("title", ""),
                ("text", "Hello baby"),
                ("language", "English"),
            ],
            "audio_file",
            "language-recording.wav",
            "audio/wav",
            wav.clone(),
        ),
        &config,
    );
    assert_eq!(recorded.status, 200);
    let recorded_body: serde_json::Value = serde_json::from_slice(&recorded.body).unwrap();
    assert_eq!(recorded_body["state"], "archived");
    assert_eq!(recorded_body["source"], "recorded");
    assert_eq!(recorded_body["text"], "Hello baby");
    assert!(recorded_body["title"]
        .as_str()
        .unwrap()
        .starts_with("recorded-english-hello-baby-"));
    assert!(recorded_body["audio_path"]
        .as_str()
        .unwrap()
        .starts_with("data/audio/draft/language/recorded-english-hello-baby-"));
    let recorded_path = recorded_body["audio_path"]
        .as_str()
        .unwrap()
        .strip_prefix("data/audio/")
        .unwrap();
    assert!(config.media_root.join(recorded_path).exists());

    let recorded_without_title = axum_request(
        &multipart_request(
            "/api/content/recordings",
            &cookie,
            vec![
                ("content_type", "language"),
                ("button_id", "4"),
                ("title", ""),
                ("text", "Bonjour bebe"),
                ("language", "French"),
            ],
            "audio_file",
            "french-recording.wav",
            "audio/wav",
            wav.clone(),
        ),
        &config,
    );
    assert_eq!(recorded_without_title.status, 200);
    let recorded_without_title_body: serde_json::Value =
        serde_json::from_slice(&recorded_without_title.body).unwrap();
    assert_eq!(recorded_without_title_body["state"], "archived");
    assert_eq!(recorded_without_title_body["text"], "Bonjour bebe");
    assert_eq!(recorded_without_title_body["language"], "French");
    assert!(recorded_without_title_body["title"]
        .as_str()
        .unwrap()
        .starts_with("recorded-french-bonjour-bebe-"));
    let french_recorded_path = recorded_without_title_body["audio_path"]
        .as_str()
        .unwrap()
        .strip_prefix("data/audio/")
        .unwrap();
    assert!(config.media_root.join(french_recorded_path).exists());

    let recorded_without_spoken_text = axum_request(
        &multipart_request(
            "/api/content/recordings",
            &cookie,
            vec![
                ("content_type", "language"),
                ("button_id", "4"),
                ("title", ""),
                ("text", ""),
                ("language", "French"),
            ],
            "audio_file",
            "french-recording.wav",
            "audio/wav",
            wav.clone(),
        ),
        &config,
    );
    assert_eq!(recorded_without_spoken_text.status, 400);

    let oversized_upload = axum_request(
        &multipart_request(
            "/api/content/uploads",
            &cookie,
            vec![
                ("content_type", "animals"),
                ("button_id", "2"),
                ("title", "Too big"),
                ("text", ""),
                ("language", ""),
            ],
            "audio_file",
            "too-big.wav",
            "audio/wav",
            vec![0_u8; MAX_AUDIO_BYTES + 1],
        ),
        &config,
    );
    assert_eq!(oversized_upload.status, 400);
    let oversized_body: serde_json::Value = serde_json::from_slice(&oversized_upload.body).unwrap();
    assert_eq!(
        oversized_body["detail"],
        "audio file must be 25 MB or smaller"
    );

    let malformed_recording = axum_request(
        &multipart_request(
            "/api/content/recordings",
            &cookie,
            vec![
                ("content_type", "language"),
                ("button_id", "1"),
                ("title", ""),
                ("text", "Broken audio"),
                ("language", "English"),
            ],
            "audio_file",
            "broken-recording.wav",
            "audio/wav",
            malformed_short_fmt_wav(),
        ),
        &config,
    );
    assert_eq!(malformed_recording.status, 400);
    let malformed_recording_body: serde_json::Value =
        serde_json::from_slice(&malformed_recording.body).unwrap();
    assert_eq!(
        malformed_recording_body["detail"],
        "recorded WAV file is malformed"
    );

    let malformed_upload = axum_request(
        &multipart_request(
            "/api/content/uploads",
            &cookie,
            vec![
                ("content_type", "animals"),
                ("button_id", "2"),
                ("title", "Broken upload"),
                ("text", ""),
                ("language", ""),
            ],
            "audio_file",
            "broken-upload.wav",
            "audio/wav",
            malformed_short_fmt_wav(),
        ),
        &config,
    );
    assert_eq!(malformed_upload.status, 400);
    let malformed_upload_body: serde_json::Value =
        serde_json::from_slice(&malformed_upload.body).unwrap();
    assert_eq!(
        malformed_upload_body["detail"],
        "recorded WAV file is malformed"
    );

    let uploaded = axum_request(
        &multipart_request(
            "/api/content/uploads",
            &cookie,
            vec![
                ("content_type", "animals"),
                ("button_id", "2"),
                ("title", "Roar"),
                ("text", ""),
                ("language", ""),
            ],
            "audio_file",
            "roar.wav",
            "audio/wav",
            wav,
        ),
        &config,
    );
    assert_eq!(uploaded.status, 200);
    let uploaded_body: serde_json::Value = serde_json::from_slice(&uploaded.body).unwrap();
    assert_eq!(uploaded_body["source"], "uploaded");
    assert_eq!(uploaded_body["title"], "Roar");
    assert_eq!(uploaded_body["text"], "Roar");
    assert!(uploaded_body["preview_url"]
        .as_str()
        .unwrap()
        .starts_with("/api/media/draft/animals/"));
    let uploaded_path = uploaded_body["audio_path"]
        .as_str()
        .unwrap()
        .strip_prefix("data/audio/")
        .unwrap();
    assert!(config.media_root.join(uploaded_path).exists());
}

struct TestDatabase {
    _dir: TempDir,
    path: PathBuf,
}

impl TestDatabase {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn test_database() -> TestDatabase {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("tcube.sqlite3");
    TestDatabase { _dir: dir, path }
}

fn test_config(database: &Path) -> AdminConfig {
    AdminConfig {
        bind: "127.0.0.1:0".to_string(),
        database: database.to_path_buf(),
        ui_dist: PathBuf::from("admin-ui"),
        media_root: database.parent().unwrap().join("media"),
        content_root: database.parent().unwrap().join("content"),
        hostname: "tcube-a7f3.local".to_string(),
        usb_address: "10.55.0.1".to_string(),
        usb_connected: true,
    }
}

#[test]
fn speech_http_client_loads_custom_root_certificate() {
    let temp_dir = TempDir::new().unwrap();
    let cert_path = temp_dir.path().join("speech-api-ca.crt");
    fs::write(
        &cert_path,
        b"-----BEGIN CERTIFICATE-----\nMIICpDCCAYwCCQDpAesS5Rc0YzANBgkqhkiG9w0BAQsFADAUMRIwEAYDVQQDDAls\nb2NhbGhvc3QwHhcNMjYwNjIyMTEyMzMyWhcNMjYwNjIzMTEyMzMyWjAUMRIwEAYD\nVQQDDAlsb2NhbGhvc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDZ\nYlRHQ24BleueDVCphzdU7ONSyLlcrR4cDlQp9ayS6z4R3ORxz18FVdABXBzBlOT6\njNLRacsgTZLOra4r+eQclls8PWj6OWkq6jFfjzYJI13rjJEwdX+k49i2riUgS3n3\nwSr7LIn56moi2r8AmGD7mZKijNXODAQ+rIT8DKKpiw7igbghUsHhD5LOZMiqNGoB\n1XGFZmYPq0F1E1rNVzpl2PEVBWxUNk9DiQPvUGNGwlcfBEniH5dfCuDfAUYeHBLY\nIPT69KoSeCoBShSvMGgewIQz16+783QAOzmC5brAZgrlKeCCNFx7QjrTouWZ1MK0\nMs+YcoQFHoEgenCs9RnZAgMBAAEwDQYJKoZIhvcNAQELBQADggEBABX4bq6VntHb\n0y52sA8w11qMR81S5IemcDzQhdwBN7Oe8Sdg3pu1xM+BuMxfmbYVP20Lt1SKIm96\n5Yuq8vjhYtvYHDFU5qkTg5vmyrJ0C+HZSlDSzGYHTKuS1tjmTOpZkUZU+SM3bXXi\nmgqwVxJ9W0dCKyKJaI5A0uPbwuGkwmOxPMoy+pqPeDY+tHrJ/bp66ew/4K2g4SDz\n/tyIpaKcKngpaVxmrml7pZ11CobuuPznIL9EGkzJQ3VRFs6CmKAbkV5X1Fx6Q1Ok\ntpfBGYghLsPt5k32bp/4+oaxGOBEV5DNSSKb8MA+dvmwWJXq0QW8G56fHlsI1q9b\nWrcxxfJMPHk=\n-----END CERTIFICATE-----\n",
    )
    .unwrap();

    let client = speech_http_client_with_ca_cert_path(Some(&cert_path)).unwrap();
    let response = client.get("https://example.com").build();
    assert!(response.is_ok());
}

#[test]
fn speech_provider_urls_must_use_https() {
    let secure = validate_speech_api_url("https://localhost:11445").unwrap();
    assert_eq!(secure.scheme(), "https");

    let insecure = validate_speech_api_url("http://localhost:11445").unwrap_err();
    assert!(insecure
        .to_string()
        .contains("speech provider URL must use https"));
}

#[test]
fn speech_provider_defaults_match_tts_caddy_contract() {
    assert_eq!(
        speech_provider_default_base_url("voxtral").unwrap(),
        "https://127.0.0.1:11445"
    );
    assert_eq!(
        speech_provider_default_base_url("vietnamese-vits").unwrap(),
        "https://127.0.0.1:11446"
    );
}

#[test]
fn speech_provider_endpoint_urls_match_tts_contract() {
    let voxtral_base = speech_provider_default_base_url("voxtral").unwrap();
    let vietnamese_base = speech_provider_default_base_url("vietnamese-vits").unwrap();

    assert_eq!(
        speech_provider_health_url(voxtral_base).unwrap(),
        "https://127.0.0.1:11445/health"
    );
    assert_eq!(
        speech_provider_speech_url("voxtral", voxtral_base).unwrap(),
        "https://127.0.0.1:11445/v1/audio/speech"
    );
    assert_eq!(
        speech_provider_voices_url("voxtral", voxtral_base).unwrap(),
        Some("https://127.0.0.1:11445/v1/audio/voices".to_string())
    );
    assert_eq!(
        speech_provider_health_url(vietnamese_base).unwrap(),
        "https://127.0.0.1:11446/health"
    );
    assert_eq!(
        speech_provider_speech_url("vietnamese-vits", vietnamese_base).unwrap(),
        "https://127.0.0.1:11446/v1/audio/speech"
    );
    assert_eq!(
        speech_provider_voices_url("vietnamese-vits", vietnamese_base).unwrap(),
        None
    );
}

#[test]
fn speech_provider_probe_rejects_insecure_urls() {
    let error = probe_speech_provider("voxtral", "http://localhost:11445").unwrap_err();

    assert!(error
        .to_string()
        .contains("speech provider URL must use https"));
}

#[test]
fn speech_provider_health_cache_reuses_recent_result() {
    let probe_count = AtomicUsize::new(0);
    let key = format!("test-provider:{}", random_token(8).unwrap());

    let first = cached_speech_provider_health(key.clone(), "voxtral".to_string(), || {
        probe_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec!["neutral_male".to_string()])
    })
    .unwrap();
    let second = cached_speech_provider_health(key, "voxtral".to_string(), || {
        probe_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec!["neutral_female".to_string()])
    })
    .unwrap();

    assert!(first.0.online);
    assert!(!first.1);
    assert_eq!(first.0.voices, vec!["neutral_male"]);
    assert!(second.0.online);
    assert!(second.1);
    assert_eq!(second.0.voices, vec!["neutral_male"]);
    assert_eq!(probe_count.load(Ordering::SeqCst), 1);
}

#[test]
fn generated_language_filename_includes_model_language_and_text() {
    let filename = generated_filename("voxtral", "French", "Bonjour bebe", "wav");

    assert!(filename.starts_with("generated-voxtral-french-bonjour-bebe-"));
    assert!(filename.ends_with(".wav"));
}

fn json_request(
    method: &str,
    path: &str,
    body: serde_json::Value,
    cookie: Option<String>,
) -> TestRequest {
    let body = serde_json::to_vec(&body).unwrap();
    let mut headers = HashMap::from([
        ("content-length".to_string(), body.len().to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ]);
    if let Some(cookie) = cookie {
        headers.insert("cookie".to_string(), cookie);
    }
    TestRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: HashMap::new(),
        headers,
        body,
    }
}

fn authed_get(path: &str, query: HashMap<String, String>, cookie: &str) -> TestRequest {
    TestRequest {
        method: "GET".to_string(),
        path: path.to_string(),
        query,
        headers: HashMap::from([("cookie".to_string(), cookie.to_string())]),
        body: Vec::new(),
    }
}

fn seed_auth_database(path: &Path, username: &str, password: &str) -> Result<String> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "create table admin_accounts (
            id text primary key,
            username text not null unique collate nocase,
            display_name text not null,
            password_hash text,
            created_at text not null,
            disabled_at text
        );
        create table devices (
            id text primary key,
            label text not null,
            token_hash text not null,
            created_at text not null,
            last_seen_at text,
            revoked_at text
        );
        create table cube_memberships (
            account_id text not null,
            device_id text not null,
            role text not null check (role in ('owner', 'manager')),
            created_at text not null,
            primary key (account_id, device_id)
        );
        create table admin_sessions (
            id text primary key,
            account_id text not null,
            token_hash text not null unique,
            created_at text not null,
            last_seen_at text not null,
            expires_at text not null,
            revoked_at text
        );
        create table cube_invitations (
            id text primary key,
            device_id text not null,
            invited_by text not null,
            role text not null check (role = 'manager'),
            code_hash text not null unique,
            created_at text not null,
            expires_at text not null,
            accepted_at text,
            accepted_by text,
            revoked_at text
        );
        create table recovery_codes (
            id text primary key,
            account_id text not null,
            code_hash text not null unique,
            created_at text not null,
            expires_at text not null,
            used_at text
        );
        create table admin_activity_events (
            id integer primary key autoincrement,
            occurred_at text not null,
            kind text not null,
            account_id text,
            button_id integer,
            content_id text,
            content_type text,
            content_title text,
            audio_path text,
            source text,
            detail text
        );
        create table device_setup (
            id integer primary key check (id = 1),
            setup_complete integer not null default 0,
            cube_name text,
            device_id text,
            admin_credential_hash text,
            wifi_ssid text,
            wifi_verified_at text,
            dashboard_host text not null default 'tcube.local',
            dashboard_ip text,
            updated_at text not null default current_timestamp
        );
        create table button_mappings (
            button_id integer primary key check (button_id between 1 and 5),
            mode text not null,
            language text,
            content_type text,
            manual_order_weight integer not null default 0,
            updated_at text not null default current_timestamp
        );
        create table content_items (
            id text primary key,
            content_type text not null,
            button_id integer,
            language text,
            title text,
            text text,
            audio_path text,
            source text not null,
            state text not null default 'active',
            order_index integer not null default 0,
            created_at text not null default current_timestamp,
            updated_at text not null default current_timestamp,
            trashed_at text,
            purge_after text
        );
        create table media_artifacts (
            id text primary key,
            content_item_id text,
            media_type text not null,
            path text,
            text text,
            state text not null default 'active'
        );",
    )?;
    let account_id = "account-1".to_string();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, ?2, 'Local owner', ?3, ?4)",
        params![account_id, username, hash_password(password)?, now()],
    )?;
    conn.execute(
        "insert into devices (id, label, token_hash, created_at) values ('device-1', 'T-Cube', 'hash', ?1)",
        [now()],
    )?;
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) values (?1, 'device-1', 'owner', ?2)",
        params![account_id, now()],
    )?;
    conn.execute(
        "insert into device_setup (id, device_id, dashboard_host) values (1, 'device-1', 'tcube.local')",
        [],
    )?;
    conn.execute(
        "insert into button_mappings (button_id, mode, language, content_type, manual_order_weight) values
         (1, 'language', 'English', 'language', 0),
         (2, 'animals', null, 'animals', 1),
         (3, 'music', null, 'music', 2)",
        [],
    )?;
    Ok(account_id)
}

fn seed_manager_account(path: &Path) -> Result<String> {
    let conn = Connection::open(path)?;
    let account_id = "manager-account".to_string();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, 'manager', 'Local manager', ?2, ?3)",
        params![account_id, hash_password("manager-password")?, now()],
    )?;
    add_cube_membership(&conn, &account_id, "device-1", CubeRole::Manager)?;
    Ok(account_id)
}

fn seed_active_content(conn: &Connection) -> Result<()> {
    conn.execute(
        "insert into content_items (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) values
         ('language-one', 'language', 1, 'English', 'Hello', 'Hello', null, 'default', 'active', 0),
         ('animal-one', 'animals', 2, null, 'Moo', 'Moo', null, 'default', 'active', 0),
         ('music-one', 'music', 3, null, 'Song', 'Song', null, 'default', 'active', 0)",
        [],
    )?;
    Ok(())
}

fn multipart_request(
    path: &str,
    cookie: &str,
    fields: Vec<(&str, &str)>,
    file_field: &str,
    filename: &str,
    content_type: &str,
    file_bytes: Vec<u8>,
) -> TestRequest {
    let boundary = "tcube-test-boundary";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{file_field}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(&file_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    TestRequest {
        method: "POST".to_string(),
        path: path.to_string(),
        query: HashMap::new(),
        headers: HashMap::from([
            ("cookie".to_string(), cookie.to_string()),
            (
                "content-type".to_string(),
                format!("multipart/form-data; boundary={boundary}"),
            ),
            ("content-length".to_string(), body.len().to_string()),
        ]),
        body,
    }
}

fn test_wav() -> Vec<u8> {
    let sample_rate = 8_000_u32;
    let samples = 800_u32;
    let data_size = samples * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for index in 0..samples {
        let value = if index % 2 == 0 {
            10_000_i16
        } else {
            -10_000_i16
        };
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn malformed_short_fmt_wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 0, 1, 0]);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&10_000_i16.to_le_bytes());
    bytes.extend_from_slice(&[0; 10]);
    bytes
}
