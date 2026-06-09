use std::{path::Path, str::FromStr};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};
use url::Url;

use crate::config::Config;

pub async fn connect(config: &Config) -> anyhow::Result<SqlitePool> {
    tokio::fs::create_dir_all(&config.data_dir).await?;
    ensure_sqlite_parent(&config.database_url)?;

    let options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;
    create_greenfield_schema(&pool).await?;
    Ok(pool)
}

async fn create_greenfield_schema(pool: &SqlitePool) -> anyhow::Result<()> {
    // Greenfield installs apply one idempotent schema instead of ordering historical migrations.
    sqlx::raw_sql(include_str!("schema.sql"))
        .execute(pool)
        .await?;
    ensure_column(
        pool,
        "sources",
        "emoji",
        "ALTER TABLE sources ADD COLUMN emoji TEXT NOT NULL DEFAULT '🔔'",
    )
    .await?;
    sqlx::query("UPDATE sources SET emoji = COALESCE(NULLIF(TRIM(emoji), ''), '🔔')")
        .execute(pool)
        .await?;
    ensure_column(
        pool,
        "requests",
        "notification_json",
        "ALTER TABLE requests ADD COLUMN notification_json TEXT NOT NULL DEFAULT '{}'",
    )
    .await?;
    seed_defaults(pool).await?;
    Ok(())
}

async fn seed_defaults(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO sources (id, name, emoji, created_at) VALUES ('default', 'Default', '🔔', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO users (id, name, created_at, updated_at)
        VALUES (
            'owner',
            'Owner',
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO user_source_subscriptions (user_id, source_id, subscribed, updated_at)
        VALUES ('owner', 'default', 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn ensure_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    statement: &str,
) -> anyhow::Result<()> {
    if !column_exists(pool, table, column).await? {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> anyhow::Result<bool> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column))
}

fn ensure_sqlite_parent(database_url: &str) -> anyhow::Result<()> {
    if database_url == "sqlite::memory:" || database_url.contains("mode=memory") {
        return Ok(());
    }
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        let path = path.split('?').next().unwrap_or(path);
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    } else if let Ok(url) = Url::parse(database_url) {
        if url.scheme() == "sqlite" {
            if let Some(path) = url.path().strip_prefix('/') {
                if let Some(parent) = Path::new(path).parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
            }
        }
    }
    Ok(())
}

