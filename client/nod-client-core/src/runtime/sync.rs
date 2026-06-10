use std::{io::ErrorKind, sync::Arc, time::Duration};

use anyhow::{Error, Result};
use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{error::ProtocolError, Error as WebSocketError, Message},
};
use url::Url;

use crate::{
    models::{NotificationDeliveryMode, RequestStatus, SyncEnvelope},
    state::StateReducer,
};

use super::{emit_to, NodClientMessage, NodClientRuntime, Outbox};

type SharedReducer = Arc<Mutex<StateReducer>>;

const RECONNECT_DELAY: Duration = Duration::from_secs(2);

impl NodClientRuntime {
    pub async fn connect_sync(&mut self) -> Result<()> {
        self.disconnect_sync().await;
        let api = self.api().await?;
        let url = api.websocket_url()?;
        let reducer = self.reducer.clone();
        let tx = self.tx.clone();

        self.sync_task = Some(tokio::spawn(run_sync_loop(url, reducer, tx)));

        Ok(())
    }

    pub async fn disconnect_sync(&mut self) {
        if let Some(task) = self.sync_task.take() {
            task.abort();
        }
        self.reducer.lock().await.mark_sync_connected(false);
        self.emit_message(NodClientMessage::SyncStatus { connected: false })
            .await;
        self.emit_state().await;
    }
}

async fn run_sync_loop(url: Url, reducer: SharedReducer, tx: Outbox) {
    let mut has_connected = false;

    loop {
        match run_connection(&url, &reducer, &tx).await {
            Ok(()) => has_connected = true,
            Err(error) if is_expected_reconnect_error(&error, has_connected) => {}
            Err(error) => {
                emit_to(
                    &tx,
                    NodClientMessage::TransientError {
                        message: error.to_string(),
                    },
                )
                .await;
            }
        }

        if reducer.lock().await.state.is_sync_connected {
            has_connected = true;
        }

        publish_connection_state(&reducer, &tx, false).await;
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

fn is_expected_reconnect_error(error: &Error, has_connected: bool) -> bool {
    let Some(websocket_error) = error.downcast_ref::<WebSocketError>() else {
        return false;
    };

    match websocket_error {
        WebSocketError::ConnectionClosed => true,
        WebSocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => true,
        WebSocketError::Io(error) if has_connected => matches!(
            error.kind(),
            ErrorKind::ConnectionAborted
                | ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::NotConnected
                | ErrorKind::TimedOut
                | ErrorKind::UnexpectedEof
        ),
        _ => false,
    }
}

async fn run_connection(url: &Url, reducer: &SharedReducer, tx: &Outbox) -> Result<()> {
    let (mut socket, _) = connect_async(url.as_str()).await?;
    publish_connection_state(reducer, tx, true).await;

    while let Some(message) = socket.next().await {
        let message = message?;
        let Some(envelope) = envelope_from_message(message) else {
            continue;
        };
        apply_sync_envelope(reducer, tx, envelope).await;
    }

    Ok(())
}

fn envelope_from_message(message: Message) -> Option<SyncEnvelope> {
    if !message.is_text() && !message.is_binary() {
        return None;
    }
    let raw = message.into_data();
    if raw.is_empty() {
        return None;
    }
    serde_json::from_slice(&raw).ok()
}

async fn apply_sync_envelope(reducer: &SharedReducer, tx: &Outbox, envelope: SyncEnvelope) {
    let notification_removal = notification_removal_for(&envelope);
    let auth_revoked = is_current_device_revoked(reducer, &envelope).await;
    let should_resync = should_resync_after(&envelope);
    let delivery_mode = notification_delivery_mode_for(&envelope);

    let candidates = {
        let mut reducer = reducer.lock().await;
        if let Some(mode) = delivery_mode {
            reducer.set_notification_delivery_mode(mode);
        }
        reducer.apply_sync_envelope(envelope)
    };

    for request in candidates {
        emit_to(
            tx,
            NodClientMessage::NotificationCandidate {
                request: Box::new(request),
            },
        )
        .await;
    }
    if let Some(request_id) = notification_removal {
        emit_to(tx, NodClientMessage::NotificationRemoved { request_id }).await;
    }

    let state = reducer.lock().await.state.clone();
    emit_to(tx, NodClientMessage::State(Box::new(state))).await;

    if auth_revoked {
        emit_to(tx, NodClientMessage::AuthRevoked {}).await;
    }
    if should_resync {
        emit_to(tx, NodClientMessage::ResyncRequired {}).await;
    }
}

async fn publish_connection_state(reducer: &SharedReducer, tx: &Outbox, connected: bool) {
    let mut reducer = reducer.lock().await;
    reducer.mark_sync_connected(connected);
    emit_to(tx, NodClientMessage::SyncStatus { connected }).await;
    emit_to(tx, NodClientMessage::State(Box::new(reducer.state.clone()))).await;
}

fn notification_removal_for(envelope: &SyncEnvelope) -> Option<String> {
    envelope
        .payload
        .request
        .as_ref()
        .filter(|request| request.status != RequestStatus::Pending)
        .map(|request| request.id.clone())
}

fn notification_delivery_mode_for(envelope: &SyncEnvelope) -> Option<NotificationDeliveryMode> {
    envelope
        .notification_delivery
        .as_ref()
        .or(envelope.payload.notification_delivery.as_ref())
        .map(|delivery| delivery.mode.clone())
}

async fn is_current_device_revoked(reducer: &SharedReducer, envelope: &SyncEnvelope) -> bool {
    if envelope.kind != "device_revoked" {
        return false;
    }
    let Some(device_id) = envelope
        .payload
        .extra
        .get("device_id")
        .and_then(|value| value.as_str())
    else {
        return false;
    };

    reducer
        .lock()
        .await
        .selected_server()
        .and_then(|server| server.device_id.as_deref())
        == Some(device_id)
}

fn should_resync_after(envelope: &SyncEnvelope) -> bool {
    matches!(
        envelope.kind.as_str(),
        "cleared" | "subscription_updated" | "resync_required"
    ) || envelope.kind.starts_with("device_")
}

#[cfg(test)]
mod tests {
    use std::io;

    use anyhow::anyhow;
    use chrono::Utc;

    use super::*;
    use crate::models::{NotificationDelivery, NotificationDeliveryMode, SyncPayload};

    fn envelope(kind: &str) -> SyncEnvelope {
        SyncEnvelope {
            kind: kind.to_string(),
            at: Utc::now(),
            notification_delivery: None,
            payload: SyncPayload::default(),
        }
    }

    #[test]
    fn resyncs_after_server_requested_snapshot_changes() {
        assert!(should_resync_after(&envelope("cleared")));
        assert!(should_resync_after(&envelope("subscription_updated")));
        assert!(should_resync_after(&envelope("resync_required")));
    }

    #[test]
    fn resyncs_after_device_lifecycle_messages() {
        assert!(should_resync_after(&envelope("device_revoked")));
        assert!(should_resync_after(&envelope("device_renamed")));
    }

    #[test]
    fn does_not_resync_after_request_creation() {
        assert!(!should_resync_after(&envelope("created")));
    }

    #[test]
    fn reads_notification_delivery_from_hello_payload() {
        let mut envelope = envelope("hello");
        envelope.payload.notification_delivery = Some(NotificationDelivery {
            mode: NotificationDeliveryMode::Push,
        });

        assert_eq!(
            notification_delivery_mode_for(&envelope),
            Some(NotificationDeliveryMode::Push)
        );
    }

    #[test]
    fn treats_reset_without_close_handshake_as_reconnect_condition() {
        let error = Error::new(WebSocketError::Protocol(
            ProtocolError::ResetWithoutClosingHandshake,
        ));

        assert!(is_expected_reconnect_error(&error, false));
    }

    #[test]
    fn treats_refused_connection_after_success_as_reconnect_condition() {
        let error = Error::new(WebSocketError::Io(io::Error::from(
            ErrorKind::ConnectionRefused,
        )));

        assert!(is_expected_reconnect_error(&error, true));
    }

    #[test]
    fn surfaces_refused_connection_before_first_success() {
        let error = Error::new(WebSocketError::Io(io::Error::from(
            ErrorKind::ConnectionRefused,
        )));

        assert!(!is_expected_reconnect_error(&error, false));
    }

    #[test]
    fn surfaces_non_websocket_sync_errors() {
        let error = anyhow!("decode drift");

        assert!(!is_expected_reconnect_error(&error, true));
    }
}
