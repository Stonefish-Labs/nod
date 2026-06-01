use sqlx::{Row, SqlitePool};

use super::{
    now_string,
    validation::{parse_optional_time, parse_time},
};
use crate::{
    auth::{generate_token, hash_secret, new_id},
    error::ApiError,
    models::{AdminIssuerToken, CreateIssuerTokenRequest, CreateIssuerTokenResponse},
};

pub async fn list_issuer_tokens_for_admin(
    pool: &SqlitePool,
) -> Result<Vec<AdminIssuerToken>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, scopes_json, created_at, revoked_at
        FROM issuer_tokens
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let scopes_json: String = row.get("scopes_json");
            Ok(AdminIssuerToken {
                id: row.get("id"),
                name: row.get("name"),
                scopes: serde_json::from_str(&scopes_json)?,
                created_at: parse_time(row.get("created_at"))?,
                revoked_at: parse_optional_time(row.get("revoked_at"))?,
            })
        })
        .collect()
}

pub async fn revoke_issuer_token(pool: &SqlitePool, token_id: &str) -> Result<(), ApiError> {
    let decision = sqlx::query(
        r#"
        UPDATE issuer_tokens
        SET revoked_at = ?
        WHERE id = ? AND revoked_at IS NULL
        "#,
    )
    .bind(now_string())
    .bind(token_id)
    .execute(pool)
    .await?;
    if decision.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}

pub async fn create_issuer_token(
    pool: &SqlitePool,
    req: CreateIssuerTokenRequest,
) -> Result<CreateIssuerTokenResponse, ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("token name is required".to_string()));
    }
    let scopes = if req.scopes.is_empty() {
        vec!["requests:write".to_string(), "requests:read".to_string()]
    } else {
        req.scopes
    };
    let token = generate_token("nod_issuer");
    let id = new_id();
    sqlx::query(
        r#"
        INSERT INTO issuer_tokens (id, name, token_hash, scopes_json, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(hash_secret(&token))
    .bind(serde_json::to_string(&scopes)?)
    .bind(now_string())
    .execute(pool)
    .await?;
    Ok(CreateIssuerTokenResponse { id, token, scopes })
}
