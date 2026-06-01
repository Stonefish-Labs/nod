use std::{path::Path, str::FromStr};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
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
    Ok(())
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
