use sqlx::{Row, SqlitePool};

use super::read::get_request;
use crate::{
    auth::new_id,
    db::{
        get_source, get_user, now_string,
        validation::{normalize_options, validate_id, validate_request},
    },
    error::ApiError,
    models::{
        CreateDecisionRequest, CreatedDecisionRequest, DecisionRequest, DecisionResolution,
        RequestOption,
    },
};

pub async fn create_request(
    pool: &SqlitePool,
    req: CreateDecisionRequest,
    created_by_issuer_token_id: Option<&str>,
) -> Result<CreatedDecisionRequest, ApiError> {
    validate_request(&req)?;
    get_source(pool, &req.source_id).await?;
    let recipients =
        resolve_request_recipients(pool, &req.source_id, req.recipients.as_ref()).await?;
    // The dedupe key is part of the issuer contract: retried creates must return the pending request.
    if let Some(key) = req.dedupe_key.as_deref() {
        if let Some(request) = find_pending_request_by_dedupe_key(pool, &req.source_id, key).await?
        {
            return Ok(CreatedDecisionRequest {
                request_id: request.id.clone(),
                deduped: true,
                request,
            });
        }
    }

    let now = now_string();
    let id = new_id();
    let decision_resolution = req
        .decision_resolution
        .unwrap_or(DecisionResolution::Shared);
    let summary = if req.summary.trim().is_empty() {
        req.body_markdown
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect()
    } else {
        req.summary
    };
    let notification = normalized_notification(req.notification);
    let options = normalize_options(req.options);

    sqlx::query(
        r#"
        INSERT INTO requests (
            id, source_id, title, summary, body_markdown, fields_json, links_json,
            image_url, notification_json, dedupe_key, expires_at, status,
            created_at, updated_at, callback_url, decision_resolution, created_by_issuer_token_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.source_id)
    .bind(req.title.trim())
    .bind(summary.trim())
    .bind(req.body_markdown.trim())
    .bind(serde_json::to_string(&req.fields)?)
    .bind(serde_json::to_string(&req.links)?)
    .bind(req.image_url)
    .bind(serde_json::to_string(&notification)?)
    .bind(req.dedupe_key)
    .bind(
        req.expires_at
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
    )
    .bind(&now)
    .bind(&now)
    .bind(req.callback_url)
    .bind(decision_resolution.as_str())
    .bind(created_by_issuer_token_id)
    .execute(pool)
    .await?;

    for user_id in &recipients {
        insert_request_recipient(pool, &id, user_id).await?;
    }

    for option in options {
        insert_option(pool, &id, option).await?;
    }

    let request = get_request(pool, &id).await?;
    Ok(CreatedDecisionRequest {
        request_id: id,
        deduped: false,
        request,
    })
}

fn normalized_notification(
    mut notification: crate::models::RequestNotification,
) -> crate::models::RequestNotification {
    notification.title = normalized_optional_text(notification.title);
    notification.body = normalized_optional_text(notification.body);
    notification
}

fn normalized_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn insert_option(
    pool: &SqlitePool,
    request_id: &str,
    option: RequestOption,
) -> Result<(), ApiError> {
    validate_id(&option.id, "option id")?;
    if option.label.trim().is_empty() {
        return Err(ApiError::BadRequest("option label is required".to_string()));
    }
    sqlx::query(
        r#"
        INSERT INTO request_options (
            request_id, option_id, kind, label, style, requires_text, text_placeholder,
            destructive, foreground, created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(request_id)
    .bind(option.id)
    .bind(option.kind.as_str())
    .bind(option.label)
    .bind(option.style)
    .bind(if option.requires_text { 1 } else { 0 })
    .bind(option.text_placeholder)
    .bind(if option.destructive { 1 } else { 0 })
    .bind(if option.foreground { 1 } else { 0 })
    .bind(now_string())
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_request_recipient(
    pool: &SqlitePool,
    request_id: &str,
    user_id: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO request_recipients (request_id, user_id, created_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .bind(now_string())
    .execute(pool)
    .await?;
    Ok(())
}

async fn resolve_request_recipients(
    pool: &SqlitePool,
    source_id: &str,
    requested: Option<&Vec<String>>,
) -> Result<Vec<String>, ApiError> {
    if let Some(requested) = requested {
        if requested.is_empty() {
            return Err(ApiError::BadRequest(
                "recipients must not be empty when provided".to_string(),
            ));
        }

        let mut recipients = Vec::with_capacity(requested.len());
        for user_id in requested {
            let user_id = user_id.trim();
            validate_id(user_id, "user id")?;
            get_user(pool, user_id).await?;
            if !recipients.iter().any(|existing| existing == user_id) {
                recipients.push(user_id.to_string());
            }
        }
        return Ok(recipients);
    }

    let rows = sqlx::query(
        r#"
        SELECT user_id
        FROM user_source_subscriptions
        WHERE source_id = ? AND subscribed = 1
        ORDER BY user_id
        "#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|row| row.get("user_id")).collect())
}

async fn find_pending_request_by_dedupe_key(
    pool: &SqlitePool,
    source_id: &str,
    dedupe_key: &str,
) -> Result<Option<DecisionRequest>, ApiError> {
    let row = sqlx::query(
        "SELECT id FROM requests WHERE source_id = ? AND dedupe_key = ? AND status = 'pending'",
    )
    .bind(source_id)
    .bind(dedupe_key)
    .fetch_optional(pool)
    .await?;
    if let Some(row) = row {
        Ok(Some(
            get_request(pool, row.get::<String, _>("id").as_str()).await?,
        ))
    } else {
        Ok(None)
    }
}
