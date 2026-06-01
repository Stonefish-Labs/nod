use nod_client_core::models::{Action, ActionKind, Channel, ClientState, Event, EventStatus};

pub fn total_pending_count(state: &ClientState) -> usize {
    state.pending_counts_by_channel.values().sum()
}

pub fn pending_count_for(channel: &Channel, state: &ClientState) -> usize {
    state
        .pending_counts_by_channel
        .get(&channel.id)
        .copied()
        .unwrap_or_default()
}

pub fn subscribed_channels(state: &ClientState) -> Vec<&Channel> {
    state
        .channels
        .iter()
        .filter(|channel| channel.subscribed)
        .collect()
}

pub fn ordered_events(events: &[Event]) -> Vec<&Event> {
    let mut ordered: Vec<_> = events.iter().collect();
    ordered.sort_by(|left, right| {
        status_rank(&left.status)
            .cmp(&status_rank(&right.status))
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });
    ordered
}

pub fn selected_channel(state: &ClientState) -> Option<&Channel> {
    state
        .selected_channel_id
        .as_deref()
        .and_then(|id| state.channels.iter().find(|channel| channel.id == id))
        .or_else(|| state.channels.iter().find(|channel| channel.subscribed))
        .or_else(|| state.channels.first())
}

pub fn selected_event(state: &ClientState) -> Option<&Event> {
    state
        .selected_event_id
        .as_deref()
        .and_then(|id| state.events.iter().find(|event| event.id == id))
        .or_else(|| ordered_events(&state.events).into_iter().next())
}

pub fn selected_server_id(state: &ClientState) -> Option<&str> {
    state
        .selected_server_id
        .as_deref()
        .or_else(|| state.servers.first().map(|server| server.id.as_str()))
}

pub fn action_requires_text(action: &Action) -> bool {
    action.requires_text
        || matches!(
            action.kind,
            ActionKind::ApproveWithText | ActionKind::RejectWithText
        )
}

pub fn action_for_kind<'a>(event: &'a Event, kind: ActionKind) -> Option<ActionChoice<'a>> {
    event
        .actions
        .iter()
        .find(|action| action.kind == kind)
        .map(ActionChoice::from_action)
        .or_else(|| default_dismiss_action(event, &kind))
}

pub fn first_text_action(event: &Event) -> Option<ActionChoice<'_>> {
    event
        .actions
        .iter()
        .find(|action| action_requires_text(action))
        .map(ActionChoice::from_action)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionChoice<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub placeholder: Option<&'a str>,
    pub requires_text: bool,
}

impl<'a> ActionChoice<'a> {
    fn from_action(action: &'a Action) -> Self {
        Self {
            id: &action.id,
            label: &action.label,
            placeholder: action.text_placeholder.as_deref(),
            requires_text: action_requires_text(action),
        }
    }
}

fn default_dismiss_action<'a>(event: &'a Event, kind: &ActionKind) -> Option<ActionChoice<'a>> {
    if *kind != ActionKind::Dismiss || !event.actions.is_empty() {
        return None;
    }

    // The core signer recognizes this implicit action for actionless events,
    // so the TUI can offer a consistent dismiss key without server metadata.
    Some(ActionChoice {
        id: "dismiss",
        label: "Dismiss",
        placeholder: None,
        requires_text: false,
    })
}

fn status_rank(status: &EventStatus) -> u8 {
    match status {
        EventStatus::Pending => 0,
        EventStatus::Resolved => 1,
        EventStatus::Expired => 1,
        EventStatus::Cancelled => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{client_state, event_with_status};
    use nod_client_core::models::EventStatus;

    #[test]
    fn ordered_events_keep_pending_before_handled() {
        let resolved = event_with_status("resolved", "default", EventStatus::Resolved);
        let pending = event_with_status("pending", "default", EventStatus::Pending);
        let events = vec![resolved, pending];

        let ordered = ordered_events(&events);

        assert_eq!(ordered[0].id, "pending");
        assert_eq!(ordered[1].id, "resolved");
    }

    #[test]
    fn selected_event_falls_back_to_first_ordered_event() {
        let mut state = client_state();
        state.events = vec![
            event_with_status("handled", "default", EventStatus::Resolved),
            event_with_status("pending", "default", EventStatus::Pending),
        ];

        let selected = selected_event(&state).map(|event| event.id.as_str());

        assert_eq!(selected, Some("pending"));
    }
}
