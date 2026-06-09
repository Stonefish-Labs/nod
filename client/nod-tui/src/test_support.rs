use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use nod_client_core::models::{
    ClientState, DecisionResolution, DevicePlatform, NotificationDeliveryMode, Request,
    RequestStatus, ServerProfile, UserDevice,
};

pub fn client_state() -> ClientState {
    ClientState {
        servers: vec![ServerProfile {
            id: "local".to_string(),
            name: "Local".to_string(),
            base_url_string: "http://localhost:8767".to_string(),
            device_name: "terminal".to_string(),
            device_id: Some("device".to_string()),
            user_id: Some("owner".to_string()),
            user_name: Some("Owner".to_string()),
        }],
        selected_server_id: Some("local".to_string()),
        current_user: None,
        devices: Vec::new(),
        channels: vec![nod_client_core::models::Channel {
            id: "default".to_string(),
            name: "Default".to_string(),
            emoji: "🔔".to_string(),
            subscribed: true,
            created_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
        }],
        pending_counts_by_channel: BTreeMap::from([("default".to_string(), 1)]),
        requests: vec![request("deploy", "default")],
        selected_channel_id: Some("default".to_string()),
        selected_request_id: Some("deploy".to_string()),
        notification_sound: "default".to_string(),
        notification_delivery_mode: NotificationDeliveryMode::Websocket,
        is_registered: true,
        is_sync_connected: false,
        last_error: None,
    }
}

pub fn request(id: &str, channel_id: &str) -> Request {
    request_with_status(id, channel_id, RequestStatus::Pending)
}

pub fn request_with_status(id: &str, channel_id: &str, status: RequestStatus) -> Request {
    Request {
        id: id.to_string(),
        request_id: id.to_string(),
        channel_id: channel_id.to_string(),
        recipients: Vec::new(),
        decision_resolution: DecisionResolution::Shared,
        title: id.to_string(),
        summary: format!("{id} summary"),
        body_markdown: format!("{id} body"),
        fields: Vec::new(),
        links: Vec::new(),
        image_url: None,
        notification: Default::default(),
        dedupe_key: None,
        expires_at: None,
        status,
        created_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
        resolved_at: None,
        decision: None,
        decisions: Vec::new(),
        callback_url: None,
        options: Vec::new(),
        request_digest: Some("digest".to_string()),
    }
}

pub fn user_device(name: &str) -> UserDevice {
    UserDevice {
        id: name.to_string(),
        user_id: "owner".to_string(),
        name: name.to_string(),
        platform: DevicePlatform::Linux,
        native_app_id: None,
        push_provider: None,
        has_push_token: false,
        has_signing_key: false,
        attestation: None,
        notification_sound: "default".to_string(),
        last_seen_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
        created_at: Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap(),
        is_current: false,
    }
}
