use std::collections::{BTreeMap, BTreeSet};

use crate::models::{
    Channel, ClientState, Event, EventStatus, NotificationDeliveryMode, ServerProfile,
    SyncEnvelope, User, UserDevice,
};

const HANDLED_EVENT_DISPLAY_LIMIT: usize = 500;
const SYNC_KIND_CREATED: &str = "created";

#[derive(Debug, Clone)]
pub struct StateReducer {
    pub state: ClientState,
    known_pending_event_channels: BTreeMap<String, String>,
    has_loaded_pending_snapshot: bool,
}

impl StateReducer {
    pub fn new(
        servers: Vec<ServerProfile>,
        selected_server_id: Option<String>,
        notification_sound: String,
    ) -> Self {
        let is_registered = !servers.is_empty();
        Self {
            state: ClientState {
                servers,
                selected_server_id,
                current_user: None,
                devices: Vec::new(),
                channels: Vec::new(),
                pending_counts_by_channel: BTreeMap::new(),
                events: Vec::new(),
                selected_channel_id: None,
                selected_event_id: None,
                notification_sound,
                notification_delivery_mode: NotificationDeliveryMode::Websocket,
                is_registered,
                is_sync_connected: false,
                last_error: None,
            },
            known_pending_event_channels: BTreeMap::new(),
            has_loaded_pending_snapshot: false,
        }
    }

    pub fn selected_server(&self) -> Option<&ServerProfile> {
        self.state
            .selected_server_id
            .as_deref()
            .and_then(|id| self.state.servers.iter().find(|server| server.id == id))
            .or_else(|| self.state.servers.first())
    }

    pub fn upsert_server(&mut self, server: ServerProfile) {
        if let Some(existing) = self
            .state
            .servers
            .iter_mut()
            .find(|existing| existing.id == server.id)
        {
            *existing = server;
        } else {
            self.state.servers.push(server);
        }
        self.state.is_registered = !self.state.servers.is_empty();
    }

    pub fn remove_server(&mut self, server_id: &str) {
        self.state.servers.retain(|server| server.id != server_id);
        if self.state.selected_server_id.as_deref() == Some(server_id) {
            self.state.selected_server_id =
                self.state.servers.first().map(|server| server.id.clone());
            self.clear_loaded_data();
        }
        self.state.is_registered = !self.state.servers.is_empty();
    }

    pub fn set_selected_server(&mut self, server_id: String) {
        self.state.selected_server_id = Some(server_id);
        self.clear_loaded_data();
    }

    pub fn apply_refresh(
        &mut self,
        current_user: Option<User>,
        devices: Vec<UserDevice>,
        channels: Vec<Channel>,
        events: Vec<Event>,
    ) -> Vec<Event> {
        self.state.current_user = current_user;
        self.state.devices = devices;
        self.state.channels = channels;
        self.ensure_selected_channel();

        let pending_events = pending_events(&events);
        self.state.pending_counts_by_channel = count_pending_by_channel(&pending_events);
        let notification_candidates = self.notification_candidates_after_refresh(&pending_events);
        self.remember_pending_events(&pending_events);

        self.state.events = self.visible_events_for_selected_channel(events);
        self.ensure_selected_event();
        self.state.last_error = None;
        notification_candidates
    }

    pub fn apply_sync_envelope(&mut self, envelope: SyncEnvelope) -> Vec<Event> {
        self.state.is_sync_connected = true;
        let mut notification_candidates = Vec::new();
        if let Some(channel) = envelope.payload.channel {
            self.upsert_channel(channel);
        }
        if let Some(event) = envelope.payload.event {
            let should_notify =
                envelope.kind == SYNC_KIND_CREATED && self.apply_event_update(event.clone());
            if should_notify {
                notification_candidates.push(event);
            }
        }
        notification_candidates
    }

    pub fn apply_event_update(&mut self, event: Event) -> bool {
        let is_new_pending = self.update_pending_tracking(&event);
        if self.state.selected_channel_id.as_deref() == Some(event.channel_id.as_str())
            || self.state.selected_channel_id.is_none()
        {
            self.upsert_visible_event(event);
        }
        is_new_pending
    }

    fn upsert_visible_event(&mut self, event: Event) {
        if let Some(existing) = self
            .state
            .events
            .iter_mut()
            .find(|existing| existing.id == event.id)
        {
            *existing = event;
        } else {
            self.state.events.insert(0, event);
        }
        let events = std::mem::take(&mut self.state.events);
        self.state.events = visible_events(events);
        self.ensure_selected_event();
    }

    fn upsert_channel(&mut self, channel: Channel) {
        if let Some(existing) = self
            .state
            .channels
            .iter_mut()
            .find(|existing| existing.id == channel.id)
        {
            *existing = channel;
        } else {
            self.state.channels.push(channel);
        }
    }

    fn update_pending_tracking(&mut self, event: &Event) -> bool {
        let previous_channel_id = self.previous_pending_channel(event).map(str::to_string);
        let is_pending = event.status == EventStatus::Pending;

        match (previous_channel_id, is_pending) {
            (None, true) => {
                self.mark_pending(event);
                true
            }
            (Some(previous_channel_id), false) => {
                self.clear_pending(&event.id, &previous_channel_id);
                false
            }
            (Some(previous_channel_id), true) => {
                self.update_pending_channel(&previous_channel_id, event);
                false
            }
            (None, false) => false,
        }
    }

    pub fn mark_sync_connected(&mut self, connected: bool) {
        self.state.is_sync_connected = connected;
    }

    pub fn set_notification_delivery_mode(&mut self, mode: NotificationDeliveryMode) {
        self.state.notification_delivery_mode = mode;
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.state.last_error = Some(error.into());
    }

    pub fn clear_loaded_data(&mut self) {
        self.state.current_user = None;
        self.state.devices.clear();
        self.state.channels.clear();
        self.state.pending_counts_by_channel.clear();
        self.state.events.clear();
        self.state.selected_channel_id = None;
        self.state.selected_event_id = None;
        self.known_pending_event_channels.clear();
        self.has_loaded_pending_snapshot = false;
    }

    fn ensure_selected_channel(&mut self) {
        let visible_channel_ids: BTreeSet<_> = self
            .state
            .channels
            .iter()
            .filter(|channel| channel.subscribed)
            .map(|channel| channel.id.as_str())
            .collect();
        let selection_is_visible = self
            .state
            .selected_channel_id
            .as_deref()
            .map(|id| visible_channel_ids.contains(id))
            .unwrap_or(false);

        if !selection_is_visible {
            self.state.selected_channel_id = self
                .state
                .channels
                .iter()
                .find(|channel| channel.subscribed)
                .map(|channel| channel.id.clone());
        }
    }

    fn notification_candidates_after_refresh(&self, pending_events: &[Event]) -> Vec<Event> {
        // The first refresh is a baseline snapshot. Only later refreshes should
        // generate local notifications for newly observed pending events.
        if !self.has_loaded_pending_snapshot {
            return Vec::new();
        }

        pending_events
            .iter()
            .filter(|event| !self.known_pending_event_channels.contains_key(&event.id))
            .cloned()
            .collect()
    }

    fn remember_pending_events(&mut self, pending_events: &[Event]) {
        self.known_pending_event_channels = pending_event_channels(pending_events);
        self.has_loaded_pending_snapshot = true;
    }

    fn previous_pending_channel(&self, event: &Event) -> Option<&str> {
        self.known_pending_event_channels
            .get(&event.id)
            .map(String::as_str)
            .or_else(|| {
                self.state
                    .events
                    .iter()
                    .find(|existing| {
                        existing.id == event.id && existing.status == EventStatus::Pending
                    })
                    .map(|existing| existing.channel_id.as_str())
            })
    }

    fn mark_pending(&mut self, event: &Event) {
        increment_pending_count(&mut self.state.pending_counts_by_channel, &event.channel_id);
        self.known_pending_event_channels
            .insert(event.id.clone(), event.channel_id.clone());
    }

    fn clear_pending(&mut self, event_id: &str, channel_id: &str) {
        decrement_pending_count(&mut self.state.pending_counts_by_channel, channel_id);
        self.known_pending_event_channels.remove(event_id);
    }

    fn update_pending_channel(&mut self, previous_channel_id: &str, event: &Event) {
        if previous_channel_id != event.channel_id {
            decrement_pending_count(
                &mut self.state.pending_counts_by_channel,
                previous_channel_id,
            );
            increment_pending_count(&mut self.state.pending_counts_by_channel, &event.channel_id);
        }
        self.known_pending_event_channels
            .insert(event.id.clone(), event.channel_id.clone());
    }

    fn visible_events_for_selected_channel(&self, events: Vec<Event>) -> Vec<Event> {
        if let Some(channel_id) = self.state.selected_channel_id.as_deref() {
            visible_events(
                events
                    .into_iter()
                    .filter(|event| event.channel_id == channel_id)
                    .collect(),
            )
        } else {
            Vec::new()
        }
    }

    fn ensure_selected_event(&mut self) {
        if self
            .state
            .selected_event_id
            .as_deref()
            .map(|id| self.state.events.iter().any(|event| event.id == id))
            .unwrap_or(false)
        {
            return;
        }
        self.state.selected_event_id = self.state.events.first().map(|event| event.id.clone());
    }
}

fn pending_events(events: &[Event]) -> Vec<Event> {
    events
        .iter()
        .filter(|event| event.status == EventStatus::Pending)
        .cloned()
        .collect()
}

fn count_pending_by_channel(events: &[Event]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events {
        increment_pending_count(&mut counts, &event.channel_id);
    }
    counts
}

fn pending_event_channels(events: &[Event]) -> BTreeMap<String, String> {
    events
        .iter()
        .map(|event| (event.id.clone(), event.channel_id.clone()))
        .collect()
}

fn increment_pending_count(counts: &mut BTreeMap<String, usize>, channel_id: &str) {
    *counts.entry(channel_id.to_string()).or_insert(0) += 1;
}

fn decrement_pending_count(counts: &mut BTreeMap<String, usize>, channel_id: &str) {
    if let Some(count) = counts.get_mut(channel_id) {
        *count = count.saturating_sub(1);
    }
    counts.retain(|_, count| *count > 0);
}

fn visible_events(mut events: Vec<Event>) -> Vec<Event> {
    events.sort_by(|lhs, rhs| {
        event_status_rank(&lhs.status)
            .cmp(&event_status_rank(&rhs.status))
            .then_with(|| rhs.created_at.cmp(&lhs.created_at))
            .then_with(|| rhs.id.cmp(&lhs.id))
    });
    let mut handled = 0;
    events
        .into_iter()
        .filter(|event| {
            if event.status == EventStatus::Pending {
                return true;
            }
            handled += 1;
            handled <= HANDLED_EVENT_DISPLAY_LIMIT
        })
        .collect()
}

fn event_status_rank(status: &EventStatus) -> u8 {
    match status {
        EventStatus::Pending => 0,
        EventStatus::Resolved | EventStatus::Expired | EventStatus::Cancelled => 1,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::models::{ActionResolution, Event};

    fn channel(id: &str, subscribed: bool) -> Channel {
        Channel {
            id: id.to_string(),
            name: id.to_string(),
            icon: String::new(),
            color: "#000000".to_string(),
            default_priority: 5,
            privacy: "private".to_string(),
            subscribed,
            created_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
        }
    }

    fn event(id: &str, channel_id: &str, status: EventStatus) -> Event {
        Event {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            recipients: Vec::new(),
            action_resolution: ActionResolution::Shared,
            title: id.to_string(),
            summary: String::new(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            priority: 5,
            privacy: "private".to_string(),
            dedupe_key: None,
            expires_at: None,
            status,
            created_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            resolved_at: None,
            result: None,
            user_results: Vec::new(),
            callback_url: None,
            actions: Vec::new(),
            request_digest: None,
        }
    }

    #[test]
    fn refresh_suppresses_initial_notification_snapshot() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_channel_id = Some("default".to_string());
        let candidates = reducer.apply_refresh(
            None,
            Vec::new(),
            Vec::new(),
            vec![event("a", "default", EventStatus::Pending)],
        );
        assert!(candidates.is_empty());
        let candidates = reducer.apply_refresh(
            None,
            Vec::new(),
            Vec::new(),
            vec![
                event("a", "default", EventStatus::Pending),
                event("b", "default", EventStatus::Pending),
            ],
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "b");
    }

    #[test]
    fn refresh_selects_first_subscribed_channel_when_selection_is_hidden() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_channel_id = Some("hidden".to_string());

        reducer.apply_refresh(
            None,
            Vec::new(),
            vec![channel("hidden", false), channel("visible", true)],
            vec![event("a", "visible", EventStatus::Pending)],
        );

        assert_eq!(
            reducer.state.selected_channel_id.as_deref(),
            Some("visible")
        );
        assert_eq!(reducer.state.events[0].channel_id, "visible");
    }

    #[test]
    fn event_update_reduces_pending_count_when_resolved() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_channel_id = Some("default".to_string());
        let pending = event("a", "default", EventStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);
        assert_eq!(
            reducer.state.pending_counts_by_channel.get("default"),
            Some(&1)
        );

        let mut resolved = pending;
        resolved.status = EventStatus::Resolved;
        reducer.apply_event_update(resolved);

        assert_eq!(reducer.state.pending_counts_by_channel.get("default"), None);
        assert_eq!(reducer.state.events[0].status, EventStatus::Resolved);
    }

    #[test]
    fn event_update_reduces_pending_count_when_cancelled() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_channel_id = Some("default".to_string());
        let pending = event("a", "default", EventStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);

        let mut cancelled = pending;
        cancelled.status = EventStatus::Cancelled;
        reducer.apply_event_update(cancelled);

        assert_eq!(reducer.state.pending_counts_by_channel.get("default"), None);
        assert_eq!(reducer.state.events[0].status, EventStatus::Cancelled);
    }

    #[test]
    fn event_update_moves_pending_count_between_channels() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_channel_id = Some("alpha".to_string());
        let pending = event("a", "alpha", EventStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);

        let mut moved = pending;
        moved.channel_id = "beta".to_string();
        reducer.apply_event_update(moved);

        assert_eq!(reducer.state.pending_counts_by_channel.get("alpha"), None);
        assert_eq!(
            reducer.state.pending_counts_by_channel.get("beta"),
            Some(&1)
        );
    }

    #[test]
    fn visible_events_keep_pending_and_limit_handled_events() {
        let mut events: Vec<_> = (0..=HANDLED_EVENT_DISPLAY_LIMIT)
            .map(|index| {
                let id = format!("resolved-{index}");
                event(&id, "default", EventStatus::Resolved)
            })
            .collect();
        events.push(event("pending", "default", EventStatus::Pending));

        let visible = visible_events(events);
        let handled_count = visible
            .iter()
            .filter(|event| event.status != EventStatus::Pending)
            .count();

        assert!(visible.iter().any(|event| event.id == "pending"));
        assert_eq!(handled_count, HANDLED_EVENT_DISPLAY_LIMIT);
    }
}
