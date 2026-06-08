use sqlx::SqlitePool;

use super::{get_device, now_string, rows::row_to_source, validation::validate_id};
use crate::{
    error::ApiError,
    models::{CreateSourceRequest, Source},
};

pub async fn list_sources(pool: &SqlitePool) -> Result<Vec<Source>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, emoji, 1 AS subscribed, created_at FROM sources ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_source).collect()
}

pub async fn list_sources_for_device(
    pool: &SqlitePool,
    device_id: &str,
) -> Result<Vec<Source>, ApiError> {
    let device = get_device(pool, device_id).await?;
    list_sources_for_user(pool, &device.user_id).await
}

pub async fn list_sources_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<Source>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.id,
            c.name,
            c.emoji,
            COALESCE(us.subscribed, 0) AS subscribed,
            c.created_at
        FROM sources c
        LEFT JOIN user_source_subscriptions us
            ON us.source_id = c.id
            AND us.user_id = ?
        ORDER BY c.name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_source).collect()
}

pub async fn create_source(
    pool: &SqlitePool,
    req: CreateSourceRequest,
) -> Result<Source, ApiError> {
    validate_id(&req.id, "source id")?;
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("source name is required".to_string()));
    }
    let now = now_string();
    sqlx::query(
        r#"
        INSERT INTO sources (id, name, emoji, created_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            emoji = excluded.emoji
        "#,
    )
    .bind(&req.id)
    .bind(req.name.trim())
    .bind(normalized_emoji(&req.emoji))
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO user_source_subscriptions (user_id, source_id, subscribed, updated_at)
        SELECT id, ?, 1, ? FROM users
        "#,
    )
    .bind(&req.id)
    .bind(now_string())
    .execute(pool)
    .await?;

    get_source(pool, &req.id).await
}

pub async fn get_source(pool: &SqlitePool, source_id: &str) -> Result<Source, ApiError> {
    let row = sqlx::query(
        "SELECT id, name, emoji, 1 AS subscribed, created_at FROM sources WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    row_to_source(row)
}

fn normalized_emoji(value: &str) -> String {
    let emoji = value.trim();
    if emoji.is_empty() {
        "🔔".to_string()
    } else {
        emoji.chars().take(8).collect()
    }
}

pub async fn delete_source(pool: &SqlitePool, source_id: &str) -> Result<(), ApiError> {
    validate_id(source_id, "source id")?;
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        DELETE FROM request_user_decisions
        WHERE request_id IN (SELECT id FROM requests WHERE source_id = ?)
        "#,
    )
    .bind(source_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM request_recipients
        WHERE request_id IN (SELECT id FROM requests WHERE source_id = ?)
        "#,
    )
    .bind(source_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM request_options
        WHERE request_id IN (SELECT id FROM requests WHERE source_id = ?)
        "#,
    )
    .bind(source_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM requests WHERE source_id = ?")
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM user_source_clears WHERE source_id = ?")
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM user_source_subscriptions WHERE source_id = ?")
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    let deleted = sqlx::query("DELETE FROM sources WHERE id = ?")
        .bind(source_id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    tx.commit().await?;
    Ok(())
}
