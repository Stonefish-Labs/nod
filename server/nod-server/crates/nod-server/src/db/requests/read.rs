use chrono::{Duration, Utc};
use sqlx::{Row, SqlitePool};

use super::rows::row_to_request;
use crate::{
    db::{get_device, rows::row_to_device, DEFAULT_USER_ID},
    error::ApiError,
    models::{DecisionRequest, DecisionResolution, Device, RequestStatus},
};

pub struct ListRequestsForDevice<'a> {
    pub device_id: &'a str,
    pub channel_id: Option<&'a str>,
    pub include_cleared: bool,
    pub handled_limit: i64,
    pub retention_days: i64,
}

pub async fn list_requests_for_device(
    pool: &SqlitePool,
    query: ListRequestsForDevice<'_>,
) -> Result<Vec<DecisionRequest>, ApiError> {
    let device = get_device(pool, query.device_id).await?;
    let cutoff = (Utc::now() - Duration::days(query.retention_days.max(1)))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // Pending work stays unbounded so a history limit cannot hide requests still waiting on this device.
    let pending_rows = sqlx::query(
        r#"
        SELECT e.id
        FROM requests e
        JOIN request_recipients er
            ON er.request_id = e.id
            AND er.user_id = ?
        JOIN user_channel_subscriptions us
            ON us.channel_id = e.channel_id
            AND us.user_id = ?
            AND us.subscribed = 1
        LEFT JOIN user_channel_clears uc
            ON uc.channel_id = e.channel_id
            AND uc.user_id = ?
        LEFT JOIN request_user_decisions eur
            ON eur.request_id = e.id
            AND eur.user_id = ?
        WHERE (? IS NULL OR e.channel_id = ?)
            AND e.created_at >= ?
            AND (? = 1 OR uc.cleared_at IS NULL OR e.created_at > uc.cleared_at)
            AND (
                (e.decision_resolution = 'shared' AND e.status = 'pending')
                OR (e.decision_resolution = 'per_user' AND e.status = 'pending' AND eur.request_id IS NULL)
            )
        ORDER BY e.created_at DESC
        "#,
    )
    .bind(&device.user_id)
    .bind(&device.user_id)
    .bind(&device.user_id)
    .bind(&device.user_id)
    .bind(query.channel_id)
    .bind(query.channel_id)
    .bind(&cutoff)
    .bind(if query.include_cleared { 1 } else { 0 })
    .fetch_all(pool)
    .await?;

    let handled_limit = query.handled_limit.clamp(0, 500);
    let handled_rows = if handled_limit > 0 {
        sqlx::query(
            r#"
            SELECT e.id
            FROM requests e
            JOIN request_recipients er
                ON er.request_id = e.id
                AND er.user_id = ?
            JOIN user_channel_subscriptions us
                ON us.channel_id = e.channel_id
                AND us.user_id = ?
                AND us.subscribed = 1
            LEFT JOIN user_channel_clears uc
                ON uc.channel_id = e.channel_id
                AND uc.user_id = ?
            LEFT JOIN request_user_decisions eur
                ON eur.request_id = e.id
                AND eur.user_id = ?
            WHERE (? IS NULL OR e.channel_id = ?)
                AND e.created_at >= ?
                AND (? = 1 OR uc.cleared_at IS NULL OR e.created_at > uc.cleared_at)
                AND (
                    (e.decision_resolution = 'shared' AND e.status != 'pending')
                    OR (e.decision_resolution = 'per_user' AND (e.status != 'pending' OR eur.request_id IS NOT NULL))
                )
            ORDER BY e.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&device.user_id)
        .bind(&device.user_id)
        .bind(&device.user_id)
        .bind(&device.user_id)
        .bind(query.channel_id)
        .bind(query.channel_id)
        .bind(&cutoff)
        .bind(if query.include_cleared { 1 } else { 0 })
        .bind(handled_limit)
        .fetch_all(pool)
        .await?
    } else {
        Vec::new()
    };

    let mut requests = Vec::with_capacity(pending_rows.len() + handled_rows.len());
    for row in pending_rows.into_iter().chain(handled_rows) {
        requests.push(
            request_for_user(pool, row.get::<String, _>("id").as_str(), &device.user_id).await?,
        );
    }
    Ok(requests)
}

pub async fn get_request(pool: &SqlitePool, request_id: &str) -> Result<DecisionRequest, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id, channel_id, title, summary, body_markdown, fields_json, links_json,
            image_url, notification_json, dedupe_key, expires_at, status, created_at,
            updated_at, resolved_at, decision_json, callback_url, decision_resolution
        FROM requests
        WHERE id = ?
        "#,
    )
    .bind(request_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    row_to_request(pool, row).await
}

pub async fn request_for_user(
    pool: &SqlitePool,
    request_id: &str,
    user_id: &str,
) -> Result<DecisionRequest, ApiError> {
    let mut request = get_request(pool, request_id).await?;
    // Stamp the digest before any per-user filtering: signatures bind to the
    // full immutable snapshot, and the sync socket delivers that canonical
    // digest, so the HTTP projection must agree rather than recompute one
    // over filtered recipients.
    request.canonical_digest = request.to_wire().request_digest;
    if request.decision_resolution == DecisionResolution::PerUser {
        // Device projections expose only the current user's decision state.
        if let Some(user_decision) = request
            .user_decisions
            .iter()
            .find(|decision| decision.user_id == user_id)
            .cloned()
        {
            request.status = RequestStatus::Resolved;
            request.resolved_at = Some(user_decision.decision.resolved_at);
            request.decision = Some(user_decision.decision);
        } else if matches!(request.status, RequestStatus::Resolved) {
            request.status = RequestStatus::Pending;
            request.resolved_at = None;
            request.decision = None;
        }
    }
    request.recipients.retain(|recipient| recipient == user_id);
    request
        .user_decisions
        .retain(|decision| decision.user_id == user_id);
    Ok(request)
}

pub async fn request_visible_to_user(
    pool: &SqlitePool,
    request_id: &str,
    user_id: &str,
) -> Result<bool, ApiError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM request_recipients WHERE request_id = ? AND user_id = ?",
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

pub async fn request_created_by_issuer_token_id(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<Option<String>, ApiError> {
    let row = sqlx::query("SELECT created_by_issuer_token_id FROM requests WHERE id = ?")
        .bind(request_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(row.get("created_by_issuer_token_id"))
}

pub async fn push_devices_for_request(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<Vec<Device>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
            d.id,
            COALESCE(d.user_id, ?) AS user_id,
            d.name,
            d.platform,
            d.native_app_id,
            d.push_provider,
            d.push_token,
            d.signing_key_id,
            d.signing_key_algorithm,
            d.signing_public_key,
            d.notification_sound,
            d.last_seen_at,
            d.created_at
        FROM devices d
        JOIN request_recipients er
            ON er.user_id = COALESCE(d.user_id, ?)
            AND er.request_id = ?
        JOIN requests e ON e.id = er.request_id
        JOIN user_channel_subscriptions us
            ON us.user_id = er.user_id
            AND us.channel_id = e.channel_id
            AND us.subscribed = 1
        WHERE d.push_token IS NOT NULL
            AND TRIM(d.push_token) != ''
            AND d.native_app_id IS NOT NULL
            AND TRIM(d.native_app_id) != ''
        "#,
    )
    .bind(DEFAULT_USER_ID)
    .bind(DEFAULT_USER_ID)
    .bind(request_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_device).collect()
}
