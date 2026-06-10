use std::fmt::Display;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::Arc;

use nod_client_core::models::ClientState;
use nod_client_core::NodClientMessage;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use nod_client_core::{NodClientRuntime, SelectRequestParams, SubmitOptionParams};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tokio::sync::Mutex;

use crate::notifier::DesktopNotifier;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::{notifier::NotificationActivation, window::focus_main_window};

const RUNTIME_MESSAGE_EVENT_NAME: &str = "nod://request";

pub(crate) async fn forward_runtime_messages(
    app: AppHandle,
    notifier: DesktopNotifier,
    mut messages: mpsc::Receiver<NodClientMessage>,
) {
    while let Some(message) = messages.recv().await {
        // Desktop side-effect failures should surface to the UI without swallowing the runtime message.
        if let Some(error_message) = desktop_side_effect_error(&app, &notifier, &message).await {
            emit_runtime_event(&app, RUNTIME_MESSAGE_EVENT_NAME, &error_message);
        }
        emit_runtime_event(&app, RUNTIME_MESSAGE_EVENT_NAME, &message);
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
        if let Some(error_message) = handle_activation(&runtime, activation).await {
            emit_runtime_event(&app, RUNTIME_MESSAGE_EVENT_NAME, &error_message);
        }
    }
}

async fn desktop_side_effect_error(
    app: &AppHandle,
    notifier: &DesktopNotifier,
    message: &NodClientMessage,
) -> Option<NodClientMessage> {
    match message {
        NodClientMessage::NotificationCandidate { request } => notifier
            .show(request)
            .await
            .err()
            .map(|error| transient_desktop_error("show desktop notification", error)),
        NodClientMessage::NotificationRemoved { request_id } => notifier
            .remove(request_id)
            .await
            .err()
            .map(|error| transient_desktop_error("remove desktop notification", error)),
        NodClientMessage::State(state) => update_badge(app, state)
            .err()
            .map(|error| transient_desktop_error("update desktop badge", error)),
        _ => None,
    }
}

// Pending count on the app's own surface: taskbar overlay badge on Windows,
// Dock badge on macOS dev builds. Zero clears the badge.
fn update_badge(app: &AppHandle, state: &ClientState) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let pending = total_pending_count(state);
    window.set_badge_count(if pending == 0 {
        None
    } else {
        Some(pending as i64)
    })
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
async fn handle_activation(
    runtime: &Arc<Mutex<NodClientRuntime>>,
    activation: NotificationActivation,
) -> Option<NodClientMessage> {
    match activation {
        NotificationActivation::Open { request_id } => {
            let request_id = request_id?;
            let mut runtime = runtime.lock().await;
            runtime
                .select_request(SelectRequestParams { request_id })
                .await
                .err()
                .map(|error| transient_desktop_error("open notification", error))
        }
        NotificationActivation::Submit {
            request_id,
            option_id,
        } => {
            let mut runtime = runtime.lock().await;
            runtime
                .submit_option(SubmitOptionParams {
                    request_id,
                    option_id,
                    text: None,
                })
                .await
                .err()
                .map(|error| transient_desktop_error("submit notification option", error))
        }
    }
}

pub(crate) fn emit_transient_error(app: &AppHandle, context: &str, error: impl Display) {
    emit_runtime_event(
        app,
        RUNTIME_MESSAGE_EVENT_NAME,
        &transient_desktop_error(context, error),
    );
}

fn emit_runtime_event<T>(app: &AppHandle, request: &str, payload: &T)
where
    T: Serialize + Clone,
{
    let _ = app.emit(request, payload.clone());
}

fn transient_desktop_error(context: &str, error: impl Display) -> NodClientMessage {
    NodClientMessage::TransientError {
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
    fn desktop_errors_become_transient_runtime_messages() {
        let message = transient_desktop_error("show desktop notification", "unsupported");

        assert!(matches!(
            message,
            NodClientMessage::TransientError { ref message }
                if message == "show desktop notification: unsupported"
        ));
    }
}
