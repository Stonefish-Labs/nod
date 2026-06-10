#[cfg(any(target_os = "linux", target_os = "windows", test))]
mod options;
mod platform;
#[cfg(any(target_os = "windows", test))]
mod windows_toast;

use nod_client_core::models::Request;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tokio::sync::mpsc;

use self::platform::{remove_notification, show_notification};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(target_os = "linux", target_os = "windows", test))]
pub(crate) enum NotificationActivation {
    Open {
        request_id: Option<String>,
    },
    Submit {
        request_id: String,
        option_id: String,
    },
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

    pub(crate) async fn show(&self, request: &Request) -> anyhow::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            return show_notification(request, self.activations.clone()).await;
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            show_notification(request).await
        }
    }

    pub(crate) async fn remove(&self, request_id: &str) -> anyhow::Result<()> {
        remove_notification(request_id).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use nod_client_core::models::{DecisionResolution, OptionKind, RequestOption, RequestStatus};

    use super::*;
    use super::{options::desktop_notification_options, windows_toast::windows_toast_xml};

    fn request(options: Vec<RequestOption>) -> Request {
        Request {
            id: "request-1".to_string(),
            request_id: "request-1".to_string(),
            channel_id: "default".to_string(),
            recipients: Vec::new(),
            decision_resolution: DecisionResolution::Shared,
            title: "Approve deploy".to_string(),
            summary: "Production deploy".to_string(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            notification: Default::default(),
            dedupe_key: None,
            expires_at: None,
            status: RequestStatus::Pending,
            created_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
            resolved_at: None,
            decision: None,
            decisions: Vec::new(),
            callback_url: None,
            options,
            request_digest: Some("digest".to_string()),
        }
    }

    fn option(id: &str, kind: OptionKind, requires_text: bool) -> RequestOption {
        RequestOption {
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
    fn default_option_dismisses_from_notification() {
        let request = request(Vec::new());
        let options = desktop_notification_options(&request);

        assert_eq!(
            options[0].activation,
            NotificationActivation::Submit {
                request_id: "request-1".to_string(),
                option_id: "dismiss".to_string()
            }
        );
    }

    #[test]
    fn simple_options_submit_from_notification() {
        let request = request(vec![option("approve", OptionKind::Approve, false)]);
        let options = desktop_notification_options(&request);

        assert_eq!(
            options[0].activation,
            NotificationActivation::Submit {
                request_id: "request-1".to_string(),
                option_id: "approve".to_string()
            }
        );
    }

    #[test]
    fn text_options_open_request_detail() {
        let request = request(vec![option(
            "approve_notes",
            OptionKind::ApproveWithText,
            true,
        )]);
        let options = desktop_notification_options(&request);

        assert_eq!(
            options[0].activation,
            NotificationActivation::Open {
                request_id: Some("request-1".to_string())
            }
        );
    }

    #[test]
    fn notification_options_are_limited_to_os_button_capacity() {
        let request = request(vec![
            option("one", OptionKind::Custom, false),
            option("two", OptionKind::Custom, false),
            option("three", OptionKind::Custom, false),
            option("four", OptionKind::Custom, false),
            option("five", OptionKind::Custom, false),
        ]);

        assert_eq!(desktop_notification_options(&request).len(), 4);
    }

    #[test]
    fn windows_xml_escapes_text_and_contains_options() {
        let request = Request {
            title: "Deploy <prod>".to_string(),
            summary: "A&B".to_string(),
            ..request(vec![option("approve", OptionKind::Approve, false)])
        };
        let xml = windows_toast_xml(&request);

        assert!(xml.contains("Deploy &lt;prod&gt;"));
        assert!(xml.contains("A&amp;B"));
        assert!(xml.contains("arguments=\"approve\""));
    }
}
