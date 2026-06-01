use chrono::{Duration, Utc};
use sqlx::{Row, SqlitePool};

use super::read::get_request;
use crate::{
    db::now_string,
    error::ApiError,
    models::{DecisionRequest, RequestStatus},
};

pub async fn cancel_request(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<DecisionRequest, ApiError> {
    let request = get_request(pool, request_id).await?;
    if !matches!(request.status, RequestStatus::Pending) {
        return Err(ApiError::Conflict(
            "request is no longer pending".to_string(),
        ));
    }

    let now = now_string();
    let updated = sqlx::query(
        "UPDATE requests SET status = 'cancelled', updated_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(now)
    .bind(request_id)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "request is no longer pending".to_string(),
        ));
    }

    get_request(pool, request_id).await
}

pub async fn expire_due_requests(pool: &SqlitePool) -> Result<Vec<DecisionRequest>, ApiError> {
    let now = now_string();
    let rows = sqlx::query(
        "SELECT id FROM requests WHERE status = 'pending' AND expires_at IS NOT NULL AND expires_at <= ?",
    )
    .bind(&now)
    .fetch_all(pool)
    .await?;
    let ids: Vec<String> = rows.into_iter().map(|row| row.get("id")).collect();
    for id in &ids {
        expire_request(pool, id).await?;
    }
    let mut expired = Vec::new();
    for id in ids {
        expired.push(get_request(pool, &id).await?);
    }
    Ok(expired)
}

pub async fn prune_retention(pool: &SqlitePool, retention_days: i64) -> Result<u64, ApiError> {
    let cutoff = (Utc::now() - Duration::days(retention_days.max(1)))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let deleted = sqlx::query("DELETE FROM requests WHERE created_at < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(deleted.rows_affected())
}

pub(super) async fn expire_request(pool: &SqlitePool, request_id: &str) -> Result<(), ApiError> {
    let now = now_string();
    sqlx::query(
        "UPDATE requests SET status = 'expired', updated_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(now)
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}
