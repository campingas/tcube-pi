use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::config::AdminConfig;

pub(crate) fn open_existing_database(path: &Path) -> Result<Option<Connection>> {
    if !path.exists() {
        return Ok(None);
    }
    Connection::open(path)
        .with_context(|| format!("failed to open SQLite database {}", path.display()))
        .map(Some)
}

pub(crate) fn open_admin_database(config: &AdminConfig) -> Result<Connection> {
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
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    migrate_admin_database(&conn, config)?;
    restrict_database_permissions(&config.database)?;
    Ok(conn)
}

pub(crate) fn migrate_admin_database(conn: &Connection, config: &AdminConfig) -> Result<()> {
    conn.execute_batch(
        "
        create table if not exists schema_migrations (
          version integer primary key,
          applied_at text not null default current_timestamp
        );

        create table if not exists device_setup (
          id integer primary key check (id = 1),
          setup_complete integer not null default 0,
          cube_name text,
          device_id text references devices(id),
          admin_credential_hash text,
          wifi_ssid text,
          wifi_verified_at text,
          dashboard_host text not null default 'tcube.local',
          dashboard_ip text,
          battery_percent integer,
          charging_state text not null default 'unknown',
          low_battery_warning integer not null default 0,
          updated_at text not null default current_timestamp
        );

        create table if not exists trusted_sessions (
          id text primary key,
          label text not null,
          created_at text not null default current_timestamp,
          last_seen_at text,
          revoked_at text
        );

        create table if not exists button_mappings (
          button_id integer primary key check (button_id between 1 and 5),
          mode text not null check (mode in ('language', 'animals', 'music', 'disabled', 'setup_help')),
          language text,
          content_type text,
          randomness_enabled integer not null default 0,
          rotation_period text not null default 'none' check (rotation_period in ('none', 'daily', 'weekly')),
          manual_order_weight integer not null default 0,
          updated_at text not null default current_timestamp
        );

        create table if not exists content_items (
          id text primary key,
          content_type text not null check (content_type in ('language', 'animals', 'music')),
          button_id integer,
          language text,
          title text,
          text text,
          audio_path text,
          source text not null check (source in ('default', 'generated', 'manual', 'uploaded', 'recorded')),
          state text not null default 'active' check (state in ('active', 'archived', 'trash')),
          order_index integer not null default 0,
          created_at text not null default current_timestamp,
          updated_at text not null default current_timestamp,
          trashed_at text,
          purge_after text,
          foreign key (button_id) references button_mappings(button_id)
        );

        create table if not exists media_artifacts (
          id text primary key,
          content_item_id text,
          media_type text not null check (media_type in ('tts_audio', 'uploaded_audio', 'recorded_audio', 'stt_text')),
          path text,
          text text,
          state text not null default 'active' check (state in ('active', 'trash', 'purged')),
          created_at text not null default current_timestamp,
          purge_after text,
          foreign key (content_item_id) references content_items(id)
        );

        create table if not exists content_jobs (
          id text primary key,
          job_type text not null check (job_type in ('language_generation', 'tts', 'stt', 'bulk_upload')),
          status text not null check (status in ('queued', 'running', 'succeeded', 'failed')),
          language text,
          count_requested integer,
          theme_tags text,
          attempts integer not null default 0,
          success_count integer not null default 0,
          failure_count integer not null default 0,
          error text,
          created_at text not null default current_timestamp,
          updated_at text not null default current_timestamp
        );

        create table if not exists setup_debug_events (
          id integer primary key autoincrement,
          occurred_at text not null default current_timestamp,
          event_type text not null,
          button_id integer,
          details text
        );

        create table if not exists button_events (
          id integer primary key autoincrement,
          occurred_at text not null,
          button_id integer not null,
          mode text not null,
          response_id text not null,
          response_text text not null
        );

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

        create table if not exists admin_accounts (
          id text primary key,
          username text not null unique collate nocase,
          display_name text not null,
          password_hash text,
          created_at text not null,
          disabled_at text
        );

        create table if not exists cube_memberships (
          account_id text not null,
          device_id text not null,
          role text not null check (role in ('owner', 'manager')),
          created_at text not null,
          primary key (account_id, device_id),
          foreign key (account_id) references admin_accounts(id),
          foreign key (device_id) references devices(id)
        );

        create table if not exists admin_sessions (
          id text primary key,
          account_id text not null,
          token_hash text not null unique,
          created_at text not null,
          last_seen_at text not null,
          expires_at text not null,
          revoked_at text,
          foreign key (account_id) references admin_accounts(id)
        );

        create table if not exists cube_invitations (
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

        create table if not exists recovery_codes (
          id text primary key,
          account_id text not null,
          code_hash text not null unique,
          created_at text not null,
          expires_at text not null,
          used_at text,
          foreign key (account_id) references admin_accounts(id)
        );
        ",
    )?;
    conn.execute(
        "insert or ignore into schema_migrations (version) values (1), (2), (3)",
        [],
    )?;
    seed_admin_defaults(conn, config)?;
    Ok(())
}

pub(crate) fn seed_admin_defaults(conn: &Connection, config: &AdminConfig) -> Result<()> {
    conn.execute(
        "insert or ignore into device_setup (id, dashboard_host) values (1, ?1)",
        [config.hostname.as_str()],
    )?;
    let mappings = [
        (1, "language", Some("English"), Some("language"), 0),
        (2, "animals", None, Some("animals"), 1),
        (3, "music", None, Some("music"), 2),
        (4, "setup_help", None, None, 3),
        (5, "setup_help", None, None, 4),
    ];
    for (button_id, mode, language, content_type, weight) in mappings {
        conn.execute(
            "insert or ignore into button_mappings \
             (button_id, mode, language, content_type, manual_order_weight) \
             values (?1, ?2, ?3, ?4, ?5)",
            params![button_id, mode, language, content_type, weight],
        )?;
    }
    seed_default_content(conn)?;
    Ok(())
}

fn seed_default_content(conn: &Connection) -> Result<()> {
    let english = [
        (
            "Hello, little one!",
            "content/audio/english/hello-litle-one.wav",
        ),
        ("Good job!", "content/audio/english/good-job.wav"),
        ("Can you clap?", "content/audio/english/can-you-clap.wav"),
        (
            "Where is your nose?",
            "content/audio/english/where-is-your-nose.wav",
        ),
        ("Good morning!", "content/audio/english/good-morning.wav"),
        (
            "Tap the button!",
            "content/audio/english/tap-the-button.wav",
        ),
        ("High five!", "content/audio/english/high-five.wav"),
        (
            "Show me your smile!",
            "content/audio/english/show-me-your-smile.wav",
        ),
        (
            "Happy play time!",
            "content/audio/english/happy-play-time.wav",
        ),
        ("One more time!", "content/audio/english/one-more-time.wav"),
    ];
    for (index, (text, audio_path)) in english.into_iter().enumerate() {
        let id = format!("english-default-{:02}", index + 1);
        conn.execute(
            "insert or ignore into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values (?1, 'language', 1, 'English', ?2, ?2, ?3, 'default', 'active', ?4)",
            params![id, text, audio_path, index as i64],
        )?;
    }

    let animals = [
        (
            "animal-pig-grunt",
            "Pig grunt",
            "Pig grunt",
            "content/audio/animals/pig-grunt.wav",
        ),
        (
            "animal-cow-moo",
            "Cow moo",
            "Cow moo",
            "content/audio/animals/cow-moo.wav",
        ),
        (
            "animal-cat-meow",
            "Cat meow",
            "Cat meow",
            "content/audio/animals/cat-meow.wav",
        ),
        (
            "animal-goat-baa",
            "Goat baa",
            "Goat baa",
            "content/audio/animals/goat-baa.wav",
        ),
        (
            "animal-hornet-hum",
            "Hornet hum",
            "Hornet hum",
            "content/audio/animals/hornet-hum.wav",
        ),
        (
            "animal-monkey-screech",
            "Monkey screech",
            "Monkey screech",
            "content/audio/animals/monkey-screech.wav",
        ),
        (
            "animal-rooster-crow",
            "Rooster crow",
            "Rooster crow",
            "content/audio/animals/rooster-crow.wav",
        ),
        (
            "animal-horse-neigh",
            "Horse neigh",
            "Horse neigh",
            "content/audio/animals/horse-neigh.wav",
        ),
        (
            "animal-cricket-screech",
            "Cricket screech",
            "Cricket screech",
            "content/audio/animals/cricket-screech.wav",
        ),
        (
            "animal-bird-squeak",
            "Bird squeak",
            "Bird squeak",
            "content/audio/animals/bird-squeak.wav",
        ),
    ];
    for (index, (id, title, text, audio_path)) in animals.into_iter().enumerate() {
        conn.execute(
            "insert or ignore into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values (?1, 'animals', 2, 'English', ?2, ?3, ?4, 'default', 'active', ?5)",
            params![id, title, text, audio_path, index as i64],
        )?;
    }

    let music = [
        ("Ba oi ba", "content/audio/music/ba-oi-ba.mp3"),
        ("Elicopter", "content/audio/music/elicopter.mp3"),
        ("Giant car", "content/audio/music/giant-car.mp3"),
        (
            "I am an excavator",
            "content/audio/music/i-am-an-excavator.mp3",
        ),
        (
            "Il etait un petit navire",
            "content/audio/music/il-etait-un-petit-navire.mp3",
        ),
        ("Police car", "content/audio/music/police-car.mp3"),
        (
            "Pomme de reinette",
            "content/audio/music/pomme-de-reinette.mp3",
        ),
        ("Race car", "content/audio/music/race-car.mp3"),
        ("Rescue team", "content/audio/music/rescue-team.mp3"),
        ("Super truck", "content/audio/music/super-truck.mp3"),
    ];
    for (index, (text, audio_path)) in music.into_iter().enumerate() {
        let id = format!("music-default-{:02}", index + 1);
        conn.execute(
            "insert or ignore into content_items \
             (id, content_type, button_id, language, title, text, audio_path, source, state, order_index) \
             values (?1, 'music', 3, null, ?2, ?2, ?3, 'default', 'active', ?4)",
            params![id, text, audio_path, index as i64],
        )?;
    }

    Ok(())
}

#[cfg(unix)]
fn restrict_database_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_database_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

pub(crate) fn table_count(conn: &Connection, table: &str) -> Result<i64> {
    if !table_exists(conn, table)? {
        return Ok(0);
    }
    let sql = format!("select count(*) from {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("failed to count {table}"))
}

pub(crate) fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists = conn.query_row(
        "select 1 from sqlite_master where type = 'table' and name = ?1",
        [table],
        |_| Ok(()),
    );
    match exists {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(error) => Err(error).context("failed to inspect SQLite schema"),
    }
}
