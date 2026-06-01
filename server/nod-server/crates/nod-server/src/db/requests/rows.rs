use sqlx::{Row, SqlitePool};

use super::super::validation::{parse_optional_time, parse_time};
use crate::{
    error::ApiError,
    models::{
        CardField, CardLink, Decision, DecisionRequest, DecisionResolution, RequestOption,
        RequestStatus, UserDecision,
    },
};

pub(super) async fn row_to_request(
    pool: &SqlitePool,
    row: sqlx::sqlite::SqliteRow,
) -> Result<DecisionRequest, ApiError> {
    let request_id: String = row.get("id");
    let recipients: Vec<String> = sqlx::query(
        r#"
        SELECT user_id
        FROM request_recipients
        WHERE request_id = ?
        ORDER BY rowid
        "#,
    )
    .bind(&request_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get("user_id"))
    .collect();

    let user_decisions = sqlx::query(
        r#"
        SELECT user_id, decision_json
        FROM request_user_decisions
        WHERE request_id = ?
        ORDER BY resolved_at
        "#,
    )
    .bind(&request_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let decision_json: String = row.get("decision_json");
        Ok(UserDecision {
            user_id: row.get("user_id"),
            decision: serde_json::from_str::<Decision>(&decision_json)?,
        })
    })
    .collect::<Result<Vec<_>, ApiError>>()?;

    let options = sqlx::query(
        r#"
        SELECT option_id, kind, label, style, requires_text, text_placeholder, destructive, foreground
        FROM request_options
        WHERE request_id = ?
        ORDER BY rowid
        "#,
    )
    .bind(&request_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| RequestOption {
        id: row.get("option_id"),
        label: row.get("label"),
        kind: crate::models::OptionKind::from(row.get::<String, _>("kind").as_str()),
        style: row.get("style"),
        requires_text: row.get::<i64, _>("requires_text") == 1,
        text_placeholder: row.get("text_placeholder"),
        destructive: row.get::<i64, _>("destructive") == 1,
        foreground: row.get::<i64, _>("foreground") == 1,
    })
    .collect();

    let fields_json: String = row.get("fields_json");
    let links_json: String = row.get("links_json");
    let decision_json: Option<String> = row.get("decision_json");

    Ok(DecisionRequest {
        id: request_id,
        source_id: row.get("source_id"),
        recipients,
        decision_resolution: DecisionResolution::from(
            row.get::<String, _>("decision_resolution").as_str(),
        ),
        title: row.get("title"),
        summary: row.get("summary"),
        body_markdown: row.get("body_markdown"),
        fields: serde_json::from_str::<Vec<CardField>>(&fields_json)?,
        links: serde_json::from_str::<Vec<CardLink>>(&links_json)?,
        image_url: row.get("image_url"),
        priority: row.get("priority"),
        privacy: row.get("privacy"),
        dedupe_key: row.get("dedupe_key"),
        expires_at: parse_optional_time(row.get("expires_at"))?,
        status: RequestStatus::from(row.get::<String, _>("status").as_str()),
        created_at: parse_time(row.get("created_at"))?,
        updated_at: parse_time(row.get("updated_at"))?,
        resolved_at: parse_optional_time(row.get("resolved_at"))?,
        decision: decision_json
            .map(|value| serde_json::from_str::<Decision>(&value))
            .transpose()?,
        user_decisions,
        callback_url: row.get("callback_url"),
        options,
    })
}
