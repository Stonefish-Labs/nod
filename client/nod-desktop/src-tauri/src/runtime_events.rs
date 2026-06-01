use std::fmt::Display;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::Arc;

use nod_client_core::models::ClientState;
use nod_client_core::NodClientEvent;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use nod_client_core::{NodClientRuntime, SelectEventParams, SubmitActionParams};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tokio::sync::Mutex;

use crate::notifier::DesktopNotifier;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::{notifier::NotificationActivation, window::focus_main_window};

const RUNTIME_EVENT_NAME: &str = "nod://event";

pub(crate) async fn forward_runtime_events(
    app: AppHandle,
    notifier: DesktopNotifier,
    mut events: mpsc::Receiver<NodClientEvent>,
) {
    while let Some(event) = events.recv().await {
        // Desktop side-effect failures should surface to the UI without swallowing the runtime event.
        if let Some(error_event) = desktop_side_effect_error(&notifier, &event).await {
            emit_runtime_event(&app, RUNTIME_EVENT_NAME, &error_event);
        }
        emit_runtime_event(&app, RUNTIME_EVENT_NAME, &event);
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) async fn handle_notification_activations(
    app: AppHandle,
    runtime: Arc<Mutex<NodClientRuntime>>,
    mut activations: mpsc::Receiver<NotificationActivation>,
) {
    while let Some(activation) = activations.recv().await {
        focus_main_window(&app);
        if let Some(error_event) = handle_activation(&runtime, activation).await {
            emit_runtime_event(&app, RUNTIME_EVENT_NAME, &error_event);
        }
    }
}

async fn desktop_side_effect_error(
    notifier: &DesktopNotifier,
    event: &NodClientEvent,
) -> Option<NodClientEvent> {
    match event {
        NodClientEvent::NotificationCandidate { event } => notifier
            .show(event)
            .await
            .err()
            .map(|error| transient_desktop_error("show desktop notification", error)),
        NodClientEvent::NotificationRemoved { event_id } => notifier
            .remove(event_id)
            .await
            .err()
            .map(|error| transient_desktop_error("remove desktop notification", error)),
        NodClientEvent::State(state) => notifier
            .set_badge_or_tray_count(total_pending_count(state))
            .await
            .err()
            .map(|error| transient_desktop_error("update desktop badge", error)),
        _ => None,
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
async fn handle_activation(
    runtime: &Arc<Mutex<NodClientRuntime>>,
    activation: NotificationActivation,
) -> Option<NodClientEvent> {
    match activation {
        NotificationActivation::Open { event_id } => {
            let event_id = event_id?;
            let mut runtime = runtime.lock().await;
            runtime
                .select_event(SelectEventParams { event_id })
                .await
                .err()
                .map(|error| transient_desktop_error("open notification", error))
        }
        NotificationActivation::Submit {
            event_id,
            action_id,
        } => {
            let mut runtime = runtime.lock().await;
            runtime
                .submit_action(SubmitActionParams {
                    event_id,
                    action_id,
                    text: None,
                })
                .await
                .err()
                .map(|error| transient_desktop_error("submit notification action", error))
        }
    }
}

fn emit_runtime_event<T>(app: &AppHandle, event: &str, payload: &T)
where
    T: Serialize + Clone,
{
    let _ = app.emit(event, payload.clone());
}

fn transient_desktop_error(context: &str, error: impl Display) -> NodClientEvent {
    NodClientEvent::TransientError {
        message: format!("{context}: {error}"),
    }
}

fn total_pending_count(state: &ClientState) -> usize {
    state.pending_counts_by_channel.values().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_errors_become_transient_runtime_events() {
        let event = transient_desktop_error("show desktop notification", "unsupported");

        assert!(matches!(
            event,
            NodClientEvent::TransientError { ref message }
                if message == "show desktop notification: unsupported"
        ));
    }
}
