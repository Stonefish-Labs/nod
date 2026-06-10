use std::time::Duration;

use serde_json::json;
use tokio::time::timeout;

use crate::{
    auth, db,
    error::ApiError,
    models::{
        CreateDecisionRequest, CreatedDecisionRequest, DecisionRequest, Device, RequestStatus,
        SubmitDecisionRequest,
    },
    state::AppState,
    sync,
    views::CallbackPayload,
};

pub(crate) struct WaitForDecision {
    pub request: DecisionRequest,
    pub timed_out: bool,
}

pub(crate) async fn create(
    state: &AppState,
    request: CreateDecisionRequest,
    created_by_issuer_token_id: Option<&str>,
) -> Result<CreatedDecisionRequest, ApiError> {
    let response = db::create_request(&state.pool, request, created_by_issuer_token_id).await?;
    if response.deduped {
        state
            .audit
            .record("request.deduped", &response.request)
            .await;
    } else {
        state
            .audit
            .record("request.created", &response.request)
            .await;
        let _ = state.sync.send(sync::request("created", &response.request));
        dispatch_pushes(state, &response.request).await;
    }
    Ok(response)
}

pub(crate) async fn cancel(
    state: &AppState,
    request_id: &str,
) -> Result<DecisionRequest, ApiError> {
    let request = db::cancel_request(&state.pool, request_id).await?;
    state.audit.record("request.cancelled", &request).await;
    let _ = state.sync.send(sync::request("cancelled", &request));
    Ok(request)
}

pub(crate) async fn record_decision(
    state: &AppState,
    device: &Device,
    request_id: &str,
    option_id: &str,
    decision: SubmitDecisionRequest,
) -> Result<DecisionRequest, ApiError> {
    // The canonical request drives audit, fanout, and callbacks: a shared
    // resolution must reach every recipient (sync::request targets
    // request.recipients), so nothing here may see a per-user projection.
    let request = db::record_decision(
        &state.pool,
        db::DecisionSubmission {
            request_id,
            option_id,
            actor_device: Some(device),
            actor_user_id: Some(&device.user_id),
            decision,
        },
    )
    .await?;
    state.audit.record("decision.recorded", &request).await;
    let envelope = if request.decision_resolution == crate::models::DecisionResolution::PerUser {
        sync::request_for_users("resolved", &request, vec![device.user_id.clone()])
    } else {
        sync::request("resolved", &request)
    };
    let _ = state.sync.send(envelope);
    dispatch_callback(state, &request).await;
    // The actor's response is their own projection, same as any device read.
    db::request_for_user(&state.pool, request_id, &device.user_id).await
}

pub(crate) async fn request_for_principal(
    state: &AppState,
    principal: &auth::Principal,
    request_id: &str,
) -> Result<DecisionRequest, ApiError> {
    match principal {
        auth::Principal::Device(device) => {
            if !db::request_visible_to_user(&state.pool, request_id, &device.user_id).await? {
                return Err(ApiError::Forbidden);
            }
            db::request_for_user(&state.pool, request_id, &device.user_id).await
        }
        _ => {
            let request = db::get_request(&state.pool, request_id).await?;
            auth::require_request_read(principal, &request.channel_id)?;
            Ok(request)
        }
    }
}

pub(crate) async fn wait_for_decision(
    state: &AppState,
    request_id: &str,
    device_user_id: Option<&str>,
    wait_for: Duration,
) -> Result<WaitForDecision, ApiError> {
    let mut rx = state.sync.subscribe();
    let poll = async {
        loop {
            let request = if let Some(user_id) = device_user_id {
                db::request_for_user(&state.pool, request_id, user_id).await?
            } else {
                db::get_request(&state.pool, request_id).await?
            };
            if !matches!(request.status, RequestStatus::Pending) {
                return Ok::<_, ApiError>(request);
            }
            // Broadcasts are the fast path; polling keeps waits correct if a message is missed.
            tokio::select! {
                _ = rx.recv() => {},
                _ = tokio::time::sleep(Duration::from_millis(500)) => {},
            }
        }
    };

    match timeout(wait_for, poll).await {
        Ok(Ok(request)) => Ok(WaitForDecision {
            request,
            timed_out: false,
        }),
        Ok(Err(err)) => Err(err),
        Err(_) => {
            let request = if let Some(user_id) = device_user_id {
                db::request_for_user(&state.pool, request_id, user_id).await?
            } else {
                db::get_request(&state.pool, request_id).await?
            };
            Ok(WaitForDecision {
                request,
                timed_out: true,
            })
        }
    }
}

async fn dispatch_pushes(state: &AppState, request: &DecisionRequest) {
    let devices = match db::push_devices_for_request(&state.pool, &request.id).await {
        Ok(devices) => devices,
        Err(err) => {
            tracing::error!(error = %err, request_id = %request.id, "failed to load push devices");
            return;
        }
    };
    // Push is best-effort; websocket sync and wait polling remain the delivery source of truth.
    for device in devices {
        let push = state.push.clone();
        let request = request.clone();
        tokio::spawn(async move {
            if let Err(err) = push.push_request(&device, &request).await {
                tracing::warn!(error = %err, device_id = %device.id, request_id = %request.id, "push failed");
            }
        });
    }
}

async fn dispatch_callback(state: &AppState, request: &DecisionRequest) {
    let Some(callback_url) = request.callback_url.as_deref() else {
        return;
    };
    let payload = CallbackPayload::from_request(request);
    match state.http.post(callback_url).json(&payload).send().await {
        Ok(response) if response.status().is_success() => {
            state.audit.record("callback.delivered", &payload).await;
        }
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::warn!(%status, text, request_id = %request.id, "callback rejected");
            state
                .audit
                .record(
                    "callback.failed",
                    &json!({ "request_id": request.id, "status": status.as_u16(), "body": text }),
                )
                .await;
        }
        Err(err) => {
            tracing::warn!(error = %err, request_id = %request.id, "callback failed");
            state
                .audit
                .record(
                    "callback.failed",
                    &json!({ "request_id": request.id, "error": err.to_string() }),
                )
                .await;
        }
    }
}
