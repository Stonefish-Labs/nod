use std::collections::{BTreeMap, BTreeSet};

use crate::models::{
    ClientState, NotificationDeliveryMode, Request, RequestStatus, ServerProfile, Source,
    SyncEnvelope, User, UserDevice,
};

const HANDLED_REQUEST_DISPLAY_LIMIT: usize = 500;
const SYNC_KIND_CREATED: &str = "created";

#[derive(Debug, Clone)]
pub struct StateReducer {
    pub state: ClientState,
    known_pending_request_sources: BTreeMap<String, String>,
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
                sources: Vec::new(),
                pending_counts_by_source: BTreeMap::new(),
                requests: Vec::new(),
                selected_source_id: None,
                selected_request_id: None,
                notification_sound,
                notification_delivery_mode: NotificationDeliveryMode::Websocket,
                is_registered,
                is_sync_connected: false,
                last_error: None,
            },
            known_pending_request_sources: BTreeMap::new(),
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
        sources: Vec<Source>,
        requests: Vec<Request>,
    ) -> Vec<Request> {
        self.state.current_user = current_user;
        self.state.devices = devices;
        self.state.sources = sources;
        self.ensure_selected_source();

        let pending_requests = pending_requests(&requests);
        self.state.pending_counts_by_source = count_pending_by_source(&pending_requests);
        let notification_candidates = self.notification_candidates_after_refresh(&pending_requests);
        self.remember_pending_requests(&pending_requests);

        self.state.requests = self.visible_requests_for_selected_source(requests);
        self.ensure_selected_request();
        self.state.last_error = None;
        notification_candidates
    }

    pub fn apply_sync_envelope(&mut self, envelope: SyncEnvelope) -> Vec<Request> {
        self.state.is_sync_connected = true;
        let mut notification_candidates = Vec::new();
        if let Some(source) = envelope.payload.source {
            self.upsert_source(source);
        }
        if let Some(request) = envelope.payload.request {
            let should_notify =
                envelope.kind == SYNC_KIND_CREATED && self.apply_request_update(request.clone());
            if should_notify {
                notification_candidates.push(request);
            }
        }
        notification_candidates
    }

    pub fn apply_request_update(&mut self, request: Request) -> bool {
        let is_new_pending = self.update_pending_tracking(&request);
        if self.state.selected_source_id.as_deref() == Some(request.source_id.as_str())
            || self.state.selected_source_id.is_none()
        {
            self.upsert_visible_request(request);
        }
        is_new_pending
    }

    fn upsert_visible_request(&mut self, request: Request) {
        if let Some(existing) = self
            .state
            .requests
            .iter_mut()
            .find(|existing| existing.id == request.id)
        {
            *existing = request;
        } else {
            self.state.requests.insert(0, request);
        }
        let requests = std::mem::take(&mut self.state.requests);
        self.state.requests = visible_requests(requests);
        self.ensure_selected_request();
    }

    fn upsert_source(&mut self, source: Source) {
        if let Some(existing) = self
            .state
            .sources
            .iter_mut()
            .find(|existing| existing.id == source.id)
        {
            *existing = source;
        } else {
            self.state.sources.push(source);
        }
    }

    fn update_pending_tracking(&mut self, request: &Request) -> bool {
        let previous_source_id = self.previous_pending_source(request).map(str::to_string);
        let is_pending = request.status == RequestStatus::Pending;

        match (previous_source_id, is_pending) {
            (None, true) => {
                self.mark_pending(request);
                true
            }
            (Some(previous_source_id), false) => {
                self.clear_pending(&request.id, &previous_source_id);
                false
            }
            (Some(previous_source_id), true) => {
                self.update_pending_source(&previous_source_id, request);
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
        self.state.sources.clear();
        self.state.pending_counts_by_source.clear();
        self.state.requests.clear();
        self.state.selected_source_id = None;
        self.state.selected_request_id = None;
        self.known_pending_request_sources.clear();
        self.has_loaded_pending_snapshot = false;
    }

    fn ensure_selected_source(&mut self) {
        let visible_source_ids: BTreeSet<_> = self
            .state
            .sources
            .iter()
            .filter(|source| source.subscribed)
            .map(|source| source.id.as_str())
            .collect();
        let selection_is_visible = self
            .state
            .selected_source_id
            .as_deref()
            .map(|id| visible_source_ids.contains(id))
            .unwrap_or(false);

        if !selection_is_visible {
            self.state.selected_source_id = self
                .state
                .sources
                .iter()
                .find(|source| source.subscribed)
                .map(|source| source.id.clone());
        }
    }

    fn notification_candidates_after_refresh(&self, pending_requests: &[Request]) -> Vec<Request> {
        // The first refresh is a baseline snapshot. Only later refreshes should
        // generate local notifications for newly observed pending requests.
        if !self.has_loaded_pending_snapshot {
            return Vec::new();
        }

        pending_requests
            .iter()
            .filter(|request| !self.known_pending_request_sources.contains_key(&request.id))
            .cloned()
            .collect()
    }

    fn remember_pending_requests(&mut self, pending_requests: &[Request]) {
        self.known_pending_request_sources = pending_request_sources(pending_requests);
        self.has_loaded_pending_snapshot = true;
    }

    fn previous_pending_source(&self, request: &Request) -> Option<&str> {
        self.known_pending_request_sources
            .get(&request.id)
            .map(String::as_str)
            .or_else(|| {
                self.state
                    .requests
                    .iter()
                    .find(|existing| {
                        existing.id == request.id && existing.status == RequestStatus::Pending
                    })
                    .map(|existing| existing.source_id.as_str())
            })
    }

    fn mark_pending(&mut self, request: &Request) {
        increment_pending_count(&mut self.state.pending_counts_by_source, &request.source_id);
        self.known_pending_request_sources
            .insert(request.id.clone(), request.source_id.clone());
    }

    fn clear_pending(&mut self, request_id: &str, source_id: &str) {
        decrement_pending_count(&mut self.state.pending_counts_by_source, source_id);
        self.known_pending_request_sources.remove(request_id);
    }

    fn update_pending_source(&mut self, previous_source_id: &str, request: &Request) {
        if previous_source_id != request.source_id {
            decrement_pending_count(&mut self.state.pending_counts_by_source, previous_source_id);
            increment_pending_count(&mut self.state.pending_counts_by_source, &request.source_id);
        }
        self.known_pending_request_sources
            .insert(request.id.clone(), request.source_id.clone());
    }

    fn visible_requests_for_selected_source(&self, requests: Vec<Request>) -> Vec<Request> {
        if let Some(source_id) = self.state.selected_source_id.as_deref() {
            visible_requests(
                requests
                    .into_iter()
                    .filter(|request| request.source_id == source_id)
                    .collect(),
            )
        } else {
            Vec::new()
        }
    }

    fn ensure_selected_request(&mut self) {
        if self
            .state
            .selected_request_id
            .as_deref()
            .map(|id| self.state.requests.iter().any(|request| request.id == id))
            .unwrap_or(false)
        {
            return;
        }
        self.state.selected_request_id = self
            .state
            .requests
            .first()
            .map(|request| request.id.clone());
    }
}

fn pending_requests(requests: &[Request]) -> Vec<Request> {
    requests
        .iter()
        .filter(|request| request.status == RequestStatus::Pending)
        .cloned()
        .collect()
}

fn count_pending_by_source(requests: &[Request]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for request in requests {
        increment_pending_count(&mut counts, &request.source_id);
    }
    counts
}

fn pending_request_sources(requests: &[Request]) -> BTreeMap<String, String> {
    requests
        .iter()
        .map(|request| (request.id.clone(), request.source_id.clone()))
        .collect()
}

fn increment_pending_count(counts: &mut BTreeMap<String, usize>, source_id: &str) {
    *counts.entry(source_id.to_string()).or_insert(0) += 1;
}

fn decrement_pending_count(counts: &mut BTreeMap<String, usize>, source_id: &str) {
    if let Some(count) = counts.get_mut(source_id) {
        *count = count.saturating_sub(1);
    }
    counts.retain(|_, count| *count > 0);
}

fn visible_requests(mut requests: Vec<Request>) -> Vec<Request> {
    requests.sort_by(|lhs, rhs| {
        request_status_rank(&lhs.status)
            .cmp(&request_status_rank(&rhs.status))
            .then_with(|| rhs.created_at.cmp(&lhs.created_at))
            .then_with(|| rhs.id.cmp(&lhs.id))
    });
    let mut handled = 0;
    requests
        .into_iter()
        .filter(|request| {
            if request.status == RequestStatus::Pending {
                return true;
            }
            handled += 1;
            handled <= HANDLED_REQUEST_DISPLAY_LIMIT
        })
        .collect()
}

fn request_status_rank(status: &RequestStatus) -> u8 {
    match status {
        RequestStatus::Pending => 0,
        RequestStatus::Resolved | RequestStatus::Expired | RequestStatus::Cancelled => 1,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::models::{DecisionResolution, Request};

    fn source(id: &str, subscribed: bool) -> Source {
        Source {
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

    fn request(id: &str, source_id: &str, status: RequestStatus) -> Request {
        Request {
            id: id.to_string(),
            request_id: id.to_string(),
            source_id: source_id.to_string(),
            recipients: Vec::new(),
            decision_resolution: DecisionResolution::Shared,
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
            decision: None,
            decisions: Vec::new(),
            callback_url: None,
            options: Vec::new(),
            request_digest: None,
        }
    }

    #[test]
    fn refresh_suppresses_initial_notification_snapshot() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_source_id = Some("default".to_string());
        let candidates = reducer.apply_refresh(
            None,
            Vec::new(),
            Vec::new(),
            vec![request("a", "default", RequestStatus::Pending)],
        );
        assert!(candidates.is_empty());
        let candidates = reducer.apply_refresh(
            None,
            Vec::new(),
            Vec::new(),
            vec![
                request("a", "default", RequestStatus::Pending),
                request("b", "default", RequestStatus::Pending),
            ],
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "b");
    }

    #[test]
    fn refresh_selects_first_subscribed_source_when_selection_is_hidden() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_source_id = Some("hidden".to_string());

        reducer.apply_refresh(
            None,
            Vec::new(),
            vec![source("hidden", false), source("visible", true)],
            vec![request("a", "visible", RequestStatus::Pending)],
        );

        assert_eq!(reducer.state.selected_source_id.as_deref(), Some("visible"));
        assert_eq!(reducer.state.requests[0].source_id, "visible");
    }

    #[test]
    fn request_update_reduces_pending_count_when_resolved() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_source_id = Some("default".to_string());
        let pending = request("a", "default", RequestStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);
        assert_eq!(
            reducer.state.pending_counts_by_source.get("default"),
            Some(&1)
        );

        let mut resolved = pending;
        resolved.status = RequestStatus::Resolved;
        reducer.apply_request_update(resolved);

        assert_eq!(reducer.state.pending_counts_by_source.get("default"), None);
        assert_eq!(reducer.state.requests[0].status, RequestStatus::Resolved);
    }

    #[test]
    fn request_update_reduces_pending_count_when_cancelled() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_source_id = Some("default".to_string());
        let pending = request("a", "default", RequestStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);

        let mut cancelled = pending;
        cancelled.status = RequestStatus::Cancelled;
        reducer.apply_request_update(cancelled);

        assert_eq!(reducer.state.pending_counts_by_source.get("default"), None);
        assert_eq!(reducer.state.requests[0].status, RequestStatus::Cancelled);
    }

    #[test]
    fn request_update_moves_pending_count_between_sources() {
        let mut reducer = StateReducer::new(Vec::new(), None, "default".to_string());
        reducer.state.selected_source_id = Some("alpha".to_string());
        let pending = request("a", "alpha", RequestStatus::Pending);
        reducer.apply_refresh(None, Vec::new(), Vec::new(), vec![pending.clone()]);

        let mut moved = pending;
        moved.source_id = "beta".to_string();
        reducer.apply_request_update(moved);

        assert_eq!(reducer.state.pending_counts_by_source.get("alpha"), None);
        assert_eq!(reducer.state.pending_counts_by_source.get("beta"), Some(&1));
    }

    #[test]
    fn visible_requests_keep_pending_and_limit_handled_requests() {
        let mut requests: Vec<_> = (0..=HANDLED_REQUEST_DISPLAY_LIMIT)
            .map(|index| {
                let id = format!("resolved-{index}");
                request(&id, "default", RequestStatus::Resolved)
            })
            .collect();
        requests.push(request("pending", "default", RequestStatus::Pending));

        let visible = visible_requests(requests);
        let handled_count = visible
            .iter()
            .filter(|request| request.status != RequestStatus::Pending)
            .count();

        assert!(visible.iter().any(|request| request.id == "pending"));
        assert_eq!(handled_count, HANDLED_REQUEST_DISPLAY_LIMIT);
    }
}
