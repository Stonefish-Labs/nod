use nod_client_core::models::Event;
use notify_rust::{Hint, Notification};
use tokio::sync::mpsc;

use crate::notifier::{actions::desktop_notification_actions, NotificationActivation};

pub(crate) async fn show_notification(
    event: &Event,
    activations: mpsc::Sender<NotificationActivation>,
) -> anyhow::Result<()> {
    let body = if event.summary.trim().is_empty() {
        event.body_markdown.as_str()
    } else {
        event.summary.as_str()
    };
    let actions = desktop_notification_actions(event);
    let mut notification = Notification::new();
    notification
        .summary(&event.title)
        .body(body)
        .appname("Nod")
        .id(stable_notification_id(&event.id))
        .hint(Hint::Category("email".to_string()));
    for action in &actions {
        notification.action(&action.id, &action.label);
    }
    let handle = notification.show()?;
    let event_id = event.id.clone();
    // notify-rust waits on a blocking DBus callback; keep that work off Tokio's async workers.
    tauri::async_runtime::spawn_blocking(move || {
        handle.wait_for_action(|action_id| {
            let activation = actions
                .iter()
                .find(|action| action.id == action_id)
                .map(|action| action.activation.clone())
                .unwrap_or(NotificationActivation::Open {
                    event_id: Some(event_id.clone()),
                });
            let _ = activations.blocking_send(activation);
        });
    });
    Ok(())
}

pub(crate) async fn remove_notification(event_id: &str) -> anyhow::Result<()> {
    let _ = notify_rust::close_notification(stable_notification_id(event_id));
    Ok(())
}

fn stable_notification_id(event_id: &str) -> u32 {
    // Linux notification replacement needs a stable numeric ID, while Nod events use strings.
    let mut hash = 2_166_136_261_u32;
    for byte in event_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}
