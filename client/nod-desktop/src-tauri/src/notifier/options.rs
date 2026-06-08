use nod_client_core::models::{OptionKind, Request, RequestOption};

use super::NotificationActivation;

const MAX_NOTIFICATION_OPTIONS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DesktopNotificationOption {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) activation: NotificationActivation,
}

pub(super) fn desktop_notification_options(request: &Request) -> Vec<DesktopNotificationOption> {
    if request.options.is_empty() {
        return vec![DesktopNotificationOption {
            id: "dismiss".to_string(),
            label: "Dismiss".to_string(),
            activation: NotificationActivation::Submit {
                request_id: request.id.clone(),
                option_id: "dismiss".to_string(),
            },
        }];
    }

    request
        .options
        .iter()
        .take(MAX_NOTIFICATION_OPTIONS)
        .map(|option| option_for_request(request, option))
        .collect()
}

fn option_for_request(request: &Request, option: &RequestOption) -> DesktopNotificationOption {
    // OS notification buttons cannot collect free-form text, so text options open the request detail view.
    let activation = if option.requires_text || requires_text(&option.kind) {
        NotificationActivation::Open {
            request_id: Some(request.id.clone()),
        }
    } else {
        NotificationActivation::Submit {
            request_id: request.id.clone(),
            option_id: option.id.clone(),
        }
    };

    DesktopNotificationOption {
        id: option.id.clone(),
        label: option.label.clone(),
        activation,
    }
}

fn requires_text(kind: &OptionKind) -> bool {
    matches!(
        kind,
        OptionKind::ApproveWithText | OptionKind::RejectWithText
    )
}
