use axum::{
    extract::State,
    http::header,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth, error::ApiError, state::AppState};

/// Baked into the binary so a downloaded release runs with no asset files on disk.
const ADMIN_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/admin.html"
));

#[derive(Debug, Deserialize)]
pub(crate) struct AdminSessionRequest {
    token: String,
}

#[derive(Debug, Serialize)]
struct AdminSessionResponse {
    ok: bool,
}

impl AdminSessionResponse {
    fn ok() -> Self {
        Self { ok: true }
    }
}

pub(crate) async fn admin_page() -> Result<Html<String>, ApiError> {
    // NOD_ADMIN_HTML_PATH is read per request so admin-panel edits show on
    // refresh during development without rebuilding the embedded copy.
    if let Ok(path) = std::env::var("NOD_ADMIN_HTML_PATH") {
        let html = tokio::fs::read_to_string(&path).await.map_err(|err| {
            tracing::error!(
                path = %path,
                error = %err,
                "failed to read NOD_ADMIN_HTML_PATH override"
            );
            ApiError::Internal("admin HTML override unavailable".to_string())
        })?;
        return Ok(Html(html));
    }

    Ok(Html(ADMIN_HTML.to_string()))
}

pub(crate) async fn create_admin_session(
    State(state): State<AppState>,
    Json(req): Json<AdminSessionRequest>,
) -> Result<Response, ApiError> {
    if !auth::admin_token_matches(req.token.trim(), state.config.admin_token()) {
        return Err(ApiError::Forbidden);
    }

    let cookie = auth::create_admin_session_cookie(state.config.admin_token());
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(AdminSessionResponse::ok()),
    )
        .into_response())
}

pub(crate) async fn delete_admin_session() -> Response {
    (
        [(header::SET_COOKIE, auth::expired_admin_session_cookie())],
        Json(AdminSessionResponse::ok()),
    )
        .into_response()
}
