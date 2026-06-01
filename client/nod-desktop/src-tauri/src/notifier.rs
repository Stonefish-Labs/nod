#[cfg(any(target_os = "linux", target_os = "windows", test))]
mod actions;
mod platform;
#[cfg(any(target_os = "windows", test))]
mod windows_toast;

use nod_client_core::models::Event;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tokio::sync::mpsc;

use self::platform::{remove_notification, set_badge_or_tray_count, show_notification};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(target_os = "linux", target_os = "windows", test))]
pub(crate) enum NotificationActivation {
    Open { event_id: Option<String> },
    Submit { event_id: String, action_id: String },
}

#[derive(Clone)]
pub(crate) struct DesktopNotifier {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    activations: mpsc::Sender<NotificationActivation>,
}

impl DesktopNotifier {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub(crate) fn new(activations: mpsc::Sender<NotificationActivation>) -> Self {
        Self { activations }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn show(&self, event: &Event) -> anyhow::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            return show_notification(event, self.activations.clone()).await;
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            show_notification(event).await
        }
    }

    pub(crate) async fn remove(&self, event_id: &str) -> anyhow::Result<()> {
        remove_notification(event_id).await
    }

    pub(crate) async fn set_badge_or_tray_count(&self, count: usize) -> anyhow::Result<()> {
        set_badge_or_tray_count(count).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use nod_client_core::models::{Action, ActionKind, ActionResolution, EventStatus};

    use super::*;
    use super::{actions::desktop_notification_actions, windows_toast::windows_toast_xml};

    fn event(actions: Vec<Action>) -> Event {
        Event {
            id: "event-1".to_string(),
            channel_id: "default".to_string(),
            recipients: Vec::new(),
            action_resolution: ActionResolution::Shared,
            title: "Approve deploy".to_string(),
            summary: "Production deploy".to_string(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            priority: 5,
            privacy: "private".to_string(),
            dedupe_key: None,
            expires_at: None,
            status: EventStatus::Pending,
            created_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
            resolved_at: None,
            result: None,
            user_results: Vec::new(),
            callback_url: None,
            actions,
            request_digest: Some("digest".to_string()),
        }
    }

    fn action(id: &str, kind: ActionKind, requires_text: bool) -> Action {
        Action {
            id: id.to_string(),
            label: id.to_string(),
            kind,
            style: "default".to_string(),
            requires_text,
            text_placeholder: None,
            destructive: false,
            foreground: false,
        }
    }

    #[test]
    fn default_action_dismisses_from_notification() {
        let event = event(Vec::new());
        let actions = desktop_notification_actions(&event);

        assert_eq!(
            actions[0].activation,
            NotificationActivation::Submit {
                event_id: "event-1".to_string(),
                action_id: "dismiss".to_string()
            }
        );
    }

    #[test]
    fn simple_actions_submit_from_notification() {
        let event = event(vec![action("approve", ActionKind::Approve, false)]);
        let actions = desktop_notification_actions(&event);

        assert_eq!(
            actions[0].activation,
            NotificationActivation::Submit {
                event_id: "event-1".to_string(),
                action_id: "approve".to_string()
            }
        );
    }

    #[test]
    fn text_actions_open_event_detail() {
        let event = event(vec![action(
            "approve_notes",
            ActionKind::ApproveWithText,
            true,
        )]);
        let actions = desktop_notification_actions(&event);

        assert_eq!(
            actions[0].activation,
            NotificationActivation::Open {
                event_id: Some("event-1".to_string())
            }
        );
    }

    #[test]
    fn notification_actions_are_limited_to_os_button_capacity() {
        let event = event(vec![
            action("one", ActionKind::Custom, false),
            action("two", ActionKind::Custom, false),
            action("three", ActionKind::Custom, false),
            action("four", ActionKind::Custom, false),
            action("five", ActionKind::Custom, false),
        ]);

        assert_eq!(desktop_notification_actions(&event).len(), 4);
    }

    #[test]
    fn windows_xml_escapes_text_and_contains_actions() {
        let event = Event {
            title: "Deploy <prod>".to_string(),
            summary: "A&B".to_string(),
            ..event(vec![action("approve", ActionKind::Approve, false)])
        };
        let xml = windows_toast_xml(&event);

        assert!(xml.contains("Deploy &lt;prod&gt;"));
        assert!(xml.contains("A&amp;B"));
        assert!(xml.contains("arguments=\"approve\""));
    }
}
