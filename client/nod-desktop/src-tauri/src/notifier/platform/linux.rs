use nod_client_core::models::Request;
use notify_rust::{Hint, Notification};
use tokio::sync::mpsc;

use crate::notifier::{options::desktop_notification_options, NotificationActivation};

pub(crate) async fn show_notification(
    request: &Request,
    activations: mpsc::Sender<NotificationActivation>,
) -> anyhow::Result<()> {
    let body = if request.summary.trim().is_empty() {
        request.body_markdown.as_str()
    } else {
        request.summary.as_str()
    };
    let options = desktop_notification_options(request);
    let mut notification = Notification::new();
    notification
        .summary(&request.title)
        .body(body)
        .appname("Nod")
        .id(stable_notification_id(&request.id))
        .hint(Hint::Category("email".to_string()));
    for option in &options {
        notification.action(&option.id, &option.label);
    }
    let handle = notification.show()?;
    let request_id = request.id.clone();
    // notify-rust waits on a blocking DBus callback; keep that work off Tokio's async workers.
    tauri::async_runtime::spawn_blocking(move || {
        handle.wait_for_action(|option_id| {
            let activation = options
                .iter()
                .find(|option| option.id == option_id)
                .map(|option| option.activation.clone())
                .unwrap_or(NotificationActivation::Open {
                    request_id: Some(request_id.clone()),
                });
            let _ = activations.blocking_send(activation);
        });
    });
    Ok(())
}

pub(crate) async fn remove_notification(request_id: &str) -> anyhow::Result<()> {
    let connection = zbus::Connection::session().await?;
    connection
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "CloseNotification",
            &(stable_notification_id(request_id)),
        )
        .await?;
    Ok(())
}

fn stable_notification_id(request_id: &str) -> u32 {
    // Linux notification replacement needs a stable numeric ID, while Nod requests use strings.
    let mut hash = 2_166_136_261_u32;
    for byte in request_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}
