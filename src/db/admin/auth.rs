use anyhow::{Context, Result};
use base64::Engine;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use scrypt::{scrypt, Params as ScryptParams};
use sha2::{Digest, Sha256};

use super::schema::{table_count, table_exists};

const SESSION_MAX_AGE_SECONDS: i64 = 90 * 24 * 60 * 60;

#[derive(Debug)]
pub(crate) struct AuthAccount {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
}

#[derive(Debug)]
pub(crate) struct AuthSession {
    pub(crate) account: AuthAccount,
}

#[derive(Debug)]
pub(crate) struct LocalCube {
    pub(crate) device_id: String,
    pub(crate) label: String,
    pub(crate) role: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CubeRole {
    Owner,
    Manager,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RoleRequirement {
    Member,
    Owner,
}

pub(crate) fn local_cubes(conn: &Connection, account_id: &str) -> Result<Vec<LocalCube>> {
    if !table_exists(conn, "devices")? {
        return Ok(Vec::new());
    }
    if table_exists(conn, "cube_memberships")? {
        let mut stmt = conn.prepare(
            "select d.id, d.label, m.role from cube_memberships m \
             join devices d on d.id = m.device_id \
             where m.account_id = ?1 and d.revoked_at is null order by d.label",
        )?;
        let rows = stmt.query_map([account_id], |row| {
            Ok(LocalCube {
                device_id: row.get(0)?,
                label: row.get(1)?,
                role: row.get(2)?,
            })
        })?;
        return rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read local cube memberships");
    }

    let mut stmt = conn.prepare(
        "select id, label from devices where revoked_at is null order by created_at limit 1",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(LocalCube {
            device_id: row.get(0)?,
            label: row.get(1)?,
            role: "owner".to_string(),
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read local cube identity")
}

pub(crate) fn first_cube_identity(conn: &Connection) -> Result<(String, String)> {
    let (device_id, cube_name) = conn
        .prepare("select device_id, cube_name from device_setup where id = 1")?
        .query_row([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })
        .optional()?
        .unwrap_or((None, None));
    Ok((
        device_id.unwrap_or_else(generate_uuid_v4),
        cube_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "T-Cube".to_string()),
    ))
}

pub(crate) fn ensure_first_account_owner_membership(
    conn: &Connection,
    account_id: &str,
) -> Result<()> {
    if !table_exists(conn, "admin_accounts")?
        || !table_exists(conn, "cube_memberships")?
        || !table_exists(conn, "device_setup")?
        || !table_exists(conn, "devices")?
    {
        return Ok(());
    }
    if table_count(conn, "admin_accounts")? != 1 {
        return Ok(());
    }
    let membership_count: i64 = conn
        .prepare("select count(*) from cube_memberships where account_id = ?1")?
        .query_row([account_id], |row| row.get(0))
        .context("failed to count cube memberships")?;
    if membership_count > 0 {
        return Ok(());
    }

    let (device_id, cube_name) = first_cube_identity(conn)?;
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
    Ok(())
}

pub(crate) fn local_device_id(conn: &Connection) -> Result<String> {
    if table_exists(conn, "device_setup")? {
        let device_id = conn
            .prepare("select device_id from device_setup where id = 1")?
            .query_row([], |row| row.get::<_, Option<String>>(0))
            .optional()?
            .flatten();
        if let Some(device_id) = device_id {
            return Ok(device_id);
        }
    }
    if table_exists(conn, "devices")? {
        let device_id = conn
            .prepare("select id from devices where revoked_at is null order by created_at limit 1")?
            .query_row([], |row| row.get::<_, String>(0))
            .optional()?;
        if let Some(device_id) = device_id {
            return Ok(device_id);
        }
    }
    anyhow::bail!("local cube is not initialized");
}

pub(crate) fn require_local_cube_role(
    conn: &Connection,
    account_id: &str,
    requirement: RoleRequirement,
) -> Result<()> {
    let device_id = match local_device_id(conn) {
        Ok(device_id) => device_id,
        Err(error) if requirement == RoleRequirement::Owner => {
            let account_count = table_count(conn, "admin_accounts")?;
            if account_count == 1 && account_by_id(conn, account_id)?.is_some() {
                return Ok(());
            }
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    let role = local_cube_role(conn, account_id, &device_id)?;
    if requirement == RoleRequirement::Owner && role != CubeRole::Owner {
        anyhow::bail!("cube owner permission required");
    }
    Ok(())
}

fn local_cube_role(conn: &Connection, account_id: &str, device_id: &str) -> Result<CubeRole> {
    if !table_exists(conn, "cube_memberships")? {
        return Ok(CubeRole::Owner);
    }
    let role = conn
        .prepare("select role from cube_memberships where account_id = ?1 and device_id = ?2")?
        .query_row(params![account_id, device_id], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .context("failed to read cube membership")?;
    match role.as_deref() {
        Some("owner") => Ok(CubeRole::Owner),
        Some("manager") => Ok(CubeRole::Manager),
        _ => anyhow::bail!("cube membership required"),
    }
}

#[cfg(test)]
pub(crate) fn add_cube_membership(
    conn: &Connection,
    account_id: &str,
    device_id: &str,
    role: CubeRole,
) -> Result<()> {
    let role = match role {
        CubeRole::Owner => "owner",
        CubeRole::Manager => "manager",
    };
    conn.execute(
        "insert into cube_memberships (account_id, device_id, role, created_at) \
         values (?1, ?2, ?3, ?4) \
         on conflict(account_id, device_id) do update set role = excluded.role",
        params![account_id, device_id, role, now()],
    )?;
    Ok(())
}

pub(crate) fn account_by_username(
    conn: &Connection,
    username: &str,
) -> Result<Option<AuthAccount>> {
    conn.prepare(
        "select id, username, display_name from admin_accounts \
         where username = ?1 collate nocase and disabled_at is null",
    )?
    .query_row([username.trim()], |row| {
        Ok(AuthAccount {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
        })
    })
    .optional()
    .context("failed to read admin account")
}

fn account_by_id(conn: &Connection, account_id: &str) -> Result<Option<AuthAccount>> {
    conn.prepare(
        "select id, username, display_name from admin_accounts \
         where id = ?1 and disabled_at is null",
    )?
    .query_row([account_id], |row| {
        Ok(AuthAccount {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
        })
    })
    .optional()
    .context("failed to read admin account")
}

pub(crate) fn normalize_username(username: &str) -> Result<String> {
    let value = username.trim().to_lowercase();
    if value.len() < 3 || value.len() > 32 {
        anyhow::bail!("username must be 3-32 letters, numbers, dots, dashes, or underscores");
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("username must be 3-32 letters, numbers, dots, dashes, or underscores");
    };
    if !first.is_ascii_alphanumeric()
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
    {
        anyhow::bail!("username must be 3-32 letters, numbers, dots, dashes, or underscores");
    }
    Ok(value)
}

pub(crate) fn account_password_hash(conn: &Connection, account_id: &str) -> Result<Option<String>> {
    conn.prepare("select password_hash from admin_accounts where id = ?1")?
        .query_row([account_id], |row| row.get(0))
        .optional()
        .context("failed to read admin password hash")
}

pub(crate) fn authenticate_session(
    conn: &Connection,
    token: Option<&str>,
) -> Result<Option<AuthSession>> {
    let Some(token) = token else {
        return Ok(None);
    };
    let Some(row) = conn
        .prepare(
            "select s.id, s.account_id, s.expires_at, a.username, a.display_name \
             from admin_sessions s join admin_accounts a on a.id = s.account_id \
             where s.token_hash = ?1 and s.revoked_at is null and a.disabled_at is null",
        )?
        .query_row([sha256_hex(token)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .optional()?
    else {
        return Ok(None);
    };
    if row.2 <= now() {
        return Ok(None);
    }

    let expires_at = session_expires_at();
    conn.execute(
        "update admin_sessions set last_seen_at = ?1, expires_at = ?2 where id = ?3",
        params![now(), expires_at, row.0],
    )?;

    Ok(Some(AuthSession {
        account: AuthAccount {
            id: row.1,
            username: row.3,
            display_name: row.4,
        },
    }))
}

pub(crate) fn create_session(conn: &Connection, account_id: &str) -> Result<String> {
    let token = random_token(32)?;
    let timestamp = now();
    conn.execute(
        "insert into admin_sessions \
         (id, account_id, token_hash, created_at, last_seen_at, expires_at) \
         values (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            random_token(16)?,
            account_id,
            sha256_hex(&token),
            timestamp,
            timestamp,
            session_expires_at()
        ],
    )?;
    Ok(token)
}

pub(crate) fn session_id_for_token(conn: &Connection, token: &str) -> Result<Option<String>> {
    conn.prepare("select id from admin_sessions where token_hash = ?1 and revoked_at is null")?
        .query_row([sha256_hex(token)], |row| row.get(0))
        .optional()
        .context("failed to read session")
}

pub(crate) fn revoke_all_sessions(conn: &Connection, account_id: &str) -> Result<()> {
    conn.execute(
        "update admin_sessions set revoked_at = ?1 where account_id = ?2 and revoked_at is null",
        params![now(), account_id],
    )?;
    Ok(())
}

pub(crate) fn verify_password(password: &str, encoded: &str) -> Result<bool> {
    let parts = encoded.split('$').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Ok(false);
    }
    let salt = hex::decode(parts[1]).context("invalid password salt")?;
    let expected = hex::decode(parts[2]).context("invalid password digest")?;
    let actual = match parts[0] {
        "scrypt" => scrypt_digest(password, &salt)?,
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(format!("{}:{password}", parts[1]));
            hasher.finalize().to_vec()
        }
        _ => return Ok(false),
    };
    Ok(constant_time_eq(&actual, &expected))
}

pub(crate) fn hash_password(password: &str) -> Result<String> {
    let mut salt = [0_u8; 16];
    getrandom::getrandom(&mut salt).context("failed to generate password salt")?;
    let digest = scrypt_digest(password, &salt)?;
    Ok(format!(
        "scrypt${}${}",
        hex::encode(salt),
        hex::encode(digest)
    ))
}

fn scrypt_digest(password: &str, salt: &[u8]) -> Result<Vec<u8>> {
    let params = ScryptParams::new(14, 8, 1, 32).context("invalid scrypt parameters")?;
    let mut output = [0_u8; 32];
    scrypt(password.as_bytes(), salt, &params, &mut output).context("failed to hash password")?;
    Ok(output.to_vec())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

pub(crate) fn random_token(length: usize) -> Result<String> {
    let mut bytes = vec![0_u8; length];
    getrandom::getrandom(&mut bytes).context("failed to generate random token")?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

pub(crate) fn generate_uuid_v4() -> String {
    let mut bytes = [0_u8; 16];
    getrandom::getrandom(&mut bytes).expect("failed to generate uuid bytes");
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub(crate) fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

pub(crate) fn session_expires_at() -> String {
    timestamp(Utc::now() + chrono::Duration::seconds(SESSION_MAX_AGE_SECONDS))
}

pub(crate) fn now() -> String {
    timestamp(Utc::now())
}

pub(crate) fn timestamp(value: chrono::DateTime<Utc>) -> String {
    value.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}
