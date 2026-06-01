use nod_client_core::models::{Action, ActionKind, Event};

use super::NotificationActivation;

const MAX_NOTIFICATION_ACTIONS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DesktopNotificationAction {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) activation: NotificationActivation,
}

pub(super) fn desktop_notification_actions(event: &Event) -> Vec<DesktopNotificationAction> {
    if event.actions.is_empty() {
        return vec![DesktopNotificationAction {
            id: "dismiss".to_string(),
            label: "Dismiss".to_string(),
            activation: NotificationActivation::Submit {
                event_id: event.id.clone(),
                action_id: "dismiss".to_string(),
            },
        }];
    }

    event
        .actions
        .iter()
        .take(MAX_NOTIFICATION_ACTIONS)
        .map(|action| action_for_event(event, action))
        .collect()
}

fn action_for_event(event: &Event, action: &Action) -> DesktopNotificationAction {
    // OS notification buttons cannot collect free-form text, so text actions open the detail view.
    let activation = if action.requires_text || requires_text(&action.kind) {
        NotificationActivation::Open {
            event_id: Some(event.id.clone()),
        }
    } else {
        NotificationActivation::Submit {
            event_id: event.id.clone(),
            action_id: action.id.clone(),
        }
    };

    DesktopNotificationAction {
        id: action.id.clone(),
        label: action.label.clone(),
        activation,
    }
}

fn requires_text(kind: &ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::ApproveWithText | ActionKind::RejectWithText
    )
}
