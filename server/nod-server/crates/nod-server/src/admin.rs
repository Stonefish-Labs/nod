use std::path::{Path, PathBuf};

use axum::{
    extract::State,
    http::header,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth, error::ApiError, state::AppState};

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
    let path = admin_html_path();
    let html = tokio::fs::read_to_string(&path).await.map_err(|err| {
        tracing::error!(
            path = %path.display(),
            error = %err,
            "failed to read admin HTML asset"
        );
        ApiError::Internal("admin HTML asset unavailable".to_string())
    })?;

    Ok(Html(html))
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

fn admin_html_path() -> PathBuf {
    let cwd_path = Path::new("assets/admin.html");
    if cwd_path.exists() {
        return cwd_path.to_path_buf();
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/admin.html")
}
