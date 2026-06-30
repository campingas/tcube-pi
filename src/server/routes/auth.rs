use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::config::AdminConfig;
use crate::db::admin::auth::{
    account_by_username, account_password_hash, authenticate_session, create_session,
    ensure_first_account_owner_membership, first_cube_identity, generate_uuid_v4, hash_password,
    local_cubes, local_device_id, normalize_username, now, random_token, require_local_cube_role,
    revoke_all_sessions, session_id_for_token, sha256_hex, timestamp, verify_password, LocalCube,
    RoleRequirement,
};
use crate::db::admin::schema::{open_admin_database, open_existing_database, table_count};
use crate::server::routes::error::SESSION_COOKIE_NAME;

const SESSION_MAX_AGE_SECONDS: i64 = 90 * 24 * 60 * 60;

#[derive(Debug, Serialize)]
pub(crate) struct AuthSessionResponse {
    authenticated: bool,
    bootstrap_required: bool,
    account: Option<AccountResponse>,
    cubes: Vec<CubeResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AccountResponse {
    id: String,
    username: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CubeResponse {
    device_id: String,
    label: String,
    role: String,
}

impl From<LocalCube> for CubeResponse {
    fn from(cube: LocalCube) -> Self {
        Self {
            device_id: cube.device_id,
            label: cube.label,
            role: cube.role,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BootstrapRequest {
    username: String,
    display_name: Option<String>,
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecoverRequest {
    code: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecoveryCodeResponse {
    code: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvitationCreateRequest {
    device_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvitationAcceptRequest {
    code: String,
    username: String,
    display_name: Option<String>,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct InvitationResponse {
    id: String,
    code: String,
    device_id: String,
    role: &'static str,
    expires_at: String,
}

pub(crate) fn auth_session(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<(AuthSessionResponse, Option<String>)> {
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok((
            AuthSessionResponse {
                authenticated: false,
                bootstrap_required: true,
                account: None,
                cubes: Vec::new(),
            },
            None,
        ));
    };

    if let Some(session) = authenticate_session(&conn, token)? {
        ensure_first_account_owner_membership(&conn, &session.account.id)?;
        let cubes = local_cubes(&conn, &session.account.id)?
            .into_iter()
            .map(Into::into)
            .collect();
        return Ok((
            AuthSessionResponse {
                authenticated: true,
                bootstrap_required: false,
                account: Some(AccountResponse {
                    id: session.account.id,
                    username: session.account.username,
                    display_name: session.account.display_name,
                }),
                cubes,
            },
            token.map(session_cookie),
        ));
    }

    let account_count = table_count(&conn, "admin_accounts")?;
    Ok((
        AuthSessionResponse {
            authenticated: false,
            bootstrap_required: account_count == 0,
            account: None,
            cubes: Vec::new(),
        },
        None,
    ))
}

pub(crate) fn login_password(
    config: &AdminConfig,
    body: LoginRequest,
) -> Result<(AuthSessionResponse, String)> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let account =
        account_by_username(&conn, &body.username)?.context("invalid username or password")?;
    let password_hash =
        account_password_hash(&conn, &account.id)?.context("invalid username or password")?;
    if !verify_password(&body.password, &password_hash)? {
        anyhow::bail!("invalid username or password");
    }
    let token = create_session(&conn, &account.id)?;
    ensure_first_account_owner_membership(&conn, &account.id)?;
    let cubes = local_cubes(&conn, &account.id)?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account.id,
                username: account.username,
                display_name: account.display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn bootstrap_owner(
    config: &AdminConfig,
    body: BootstrapRequest,
) -> Result<(AuthSessionResponse, String)> {
    let username = normalize_username(&body.username)?;
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let display_name = body
        .display_name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let display_name = if display_name.is_empty() {
        username.clone()
    } else {
        display_name
    };

    let conn = open_admin_database(config)?;
    let account_count = table_count(&conn, "admin_accounts")?;
    if account_count > 0 {
        anyhow::bail!("local owner already exists");
    }

    let account_id = generate_uuid_v4();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            account_id,
            username,
            display_name,
            hash_password(&body.password)?,
            now()
        ],
    )?;

    let (device_id, cube_name) = first_cube_identity(&conn)?;
    conn.execute(
        "insert into devices (id, label, token_hash, created_at, revoked_at) \
         values (?1, ?2, ?3, ?4, null) \
         on conflict(id) do update set label = excluded.label",
        params![device_id, cube_name, "0".repeat(64), now()],
    )?;
    conn.execute(
        "update device_setup set device_id = ?1, updated_at = ?2 where id = 1",
        params![device_id, now()],
    )?;
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, 'owner', ?3)",
        params![account_id, device_id, now()],
    )?;

    let token = create_session(&conn, &account_id)?;
    let cubes = local_cubes(&conn, &account_id)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account_id,
                username,
                display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn recover_password(config: &AdminConfig, body: RecoverRequest) -> Result<()> {
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let code_hash = sha256_hex(&body.code);
    let row = conn
        .prepare(
            "select id, account_id from recovery_codes \
             where code_hash = ?1 and used_at is null and expires_at > ?2",
        )?
        .query_row(params![code_hash, now()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?
        .context("recovery code is invalid or expired")?;
    let password_hash = hash_password(&body.password)?;

    conn.execute(
        "update admin_accounts set password_hash = ?1 where id = ?2",
        params![password_hash, row.1],
    )?;
    conn.execute(
        "update recovery_codes set used_at = ?1 where id = ?2",
        params![now(), row.0],
    )?;
    revoke_all_sessions(&conn, &row.1)?;
    Ok(())
}

pub(crate) fn create_recovery_code(
    config: &AdminConfig,
    token: Option<&str>,
) -> Result<RecoveryCodeResponse> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    let created_at = now();
    conn.execute(
        "update recovery_codes set used_at = ?1 where account_id = ?2 and used_at is null",
        params![created_at, session.account.id],
    )?;
    let code = random_token(24)?;
    let expires_at = timestamp(Utc::now() + chrono::Duration::days(30));
    conn.execute(
        "insert into recovery_codes (id, account_id, code_hash, created_at, expires_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            generate_uuid_v4(),
            session.account.id,
            sha256_hex(&code),
            created_at,
            expires_at
        ],
    )?;
    Ok(RecoveryCodeResponse { code, expires_at })
}

pub(crate) fn create_invitation(
    config: &AdminConfig,
    token: Option<&str>,
    body: InvitationCreateRequest,
) -> Result<InvitationResponse> {
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let Some(session) = authenticate_session(&conn, token)? else {
        anyhow::bail!("authentication required");
    };
    let device_id = body.device_id.trim();
    if device_id.is_empty() {
        anyhow::bail!("device_id is required");
    }
    let local_device_id = local_device_id(&conn)?;
    if device_id != local_device_id {
        anyhow::bail!("manager invitations can only target the local cube");
    }
    require_local_cube_role(&conn, &session.account.id, RoleRequirement::Owner)?;

    let id = generate_uuid_v4();
    let code = random_token(24)?;
    let expires_at = timestamp(Utc::now() + chrono::Duration::days(7));
    conn.execute(
        "insert into cube_invitations \
         (id, device_id, invited_by, role, code_hash, created_at, expires_at) \
         values (?1, ?2, ?3, 'manager', ?4, ?5, ?6)",
        params![
            id,
            device_id,
            session.account.id,
            sha256_hex(&code),
            now(),
            expires_at
        ],
    )?;
    Ok(InvitationResponse {
        id,
        code,
        device_id: device_id.to_string(),
        role: "manager",
        expires_at,
    })
}

pub(crate) fn accept_invitation(
    config: &AdminConfig,
    body: InvitationAcceptRequest,
) -> Result<(AuthSessionResponse, String)> {
    let username = normalize_username(&body.username)?;
    if body.password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let display_name = body
        .display_name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let display_name = if display_name.is_empty() {
        username.clone()
    } else {
        display_name
    };
    let conn = Connection::open(&config.database).with_context(|| {
        format!(
            "failed to open SQLite database {}",
            config.database.display()
        )
    })?;
    let code_hash = sha256_hex(body.code.trim());
    let Some(invitation) = conn
        .prepare(
            "select id, device_id from cube_invitations \
             where code_hash = ?1 and accepted_at is null and revoked_at is null and expires_at > ?2",
        )?
        .query_row(params![code_hash, now()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?
    else {
        anyhow::bail!("invitation is invalid or expired");
    };

    let account_id = generate_uuid_v4();
    conn.execute(
        "insert into admin_accounts (id, username, display_name, password_hash, created_at) \
         values (?1, ?2, ?3, ?4, ?5)",
        params![
            account_id,
            username,
            display_name,
            hash_password(&body.password)?,
            now()
        ],
    )
    .context("failed to create manager account")?;
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, 'manager', ?3)",
        params![account_id, invitation.1, now()],
    )?;
    conn.execute(
        "update cube_invitations set accepted_at = ?1, accepted_by = ?2 where id = ?3",
        params![now(), account_id, invitation.0],
    )?;
    let token = create_session(&conn, &account_id)?;
    let cubes = local_cubes(&conn, &account_id)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok((
        AuthSessionResponse {
            authenticated: true,
            bootstrap_required: false,
            account: Some(AccountResponse {
                id: account_id,
                username,
                display_name,
            }),
            cubes,
        },
        session_cookie(&token),
    ))
}

pub(crate) fn logout(config: &AdminConfig, token: Option<&str>) -> Result<()> {
    let Some(token) = token else {
        return Ok(());
    };
    let Some(conn) = open_existing_database(&config.database)? else {
        return Ok(());
    };
    if let Some(session_id) = session_id_for_token(&conn, token)? {
        conn.execute(
            "update admin_sessions set revoked_at = ?1 where id = ?2",
            params![now(), session_id],
        )?;
    }
    Ok(())
}

pub(crate) fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={SESSION_MAX_AGE_SECONDS}"
    )
}

pub(crate) fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
}
