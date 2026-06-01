use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Ios,
    Macos,
    Watchos,
    Windows,
    Linux,
    Unknown,
}

impl DevicePlatform {
    pub fn current_desktop() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub default_priority: i64,
    pub privacy: String,
    #[serde(default = "default_true")]
    pub subscribed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub base_url_string: String,
    pub device_name: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserDevice {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub platform: DevicePlatform,
    #[serde(default)]
    pub native_app_id: Option<String>,
    pub push_provider: Option<String>,
    pub has_push_token: bool,
    pub notification_sound: String,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryMode {
    Push,
    #[default]
    Websocket,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationDelivery {
    #[serde(default)]
    pub mode: NotificationDeliveryMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Approve,
    ApproveWithText,
    Reject,
    RejectWithText,
    Dismiss,
    Open,
    Custom,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::ApproveWithText => "approve_with_text",
            Self::Reject => "reject",
            Self::RejectWithText => "reject_with_text",
            Self::Dismiss => "dismiss",
            Self::Open => "open",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub kind: ActionKind,
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default)]
    pub requires_text: bool,
    #[serde(default)]
    pub text_placeholder: Option<String>,
    #[serde(default)]
    pub destructive: bool,
    #[serde(default)]
    pub foreground: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Pending,
    Resolved,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventResult {
    // The server's decision payload still uses request/option field names.
    // Client code uses event/action names everywhere else.
    #[serde(alias = "request_id")]
    pub event_id: String,
    #[serde(alias = "option_id")]
    pub action_id: String,
    #[serde(alias = "option_kind")]
    pub action_kind: ActionKind,
    #[serde(alias = "option_label")]
    pub action_label: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub actor_user_id: Option<String>,
    #[serde(default)]
    pub actor_device_id: Option<String>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventUserResult {
    pub user_id: String,
    #[serde(alias = "decision")]
    pub result: EventResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionResolution {
    Shared,
    PerUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    #[serde(alias = "source_id")]
    pub channel_id: String,
    #[serde(default)]
    pub recipients: Vec<String>,
    #[serde(default = "default_action_resolution", alias = "decision_resolution")]
    pub action_resolution: ActionResolution,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub image_url: Option<String>,
    pub priority: i64,
    pub privacy: String,
    #[serde(default)]
    pub dedupe_key: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub status: EventStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(alias = "decision")]
    pub result: Option<EventResult>,
    #[serde(default)]
    #[serde(alias = "decisions")]
    pub user_results: Vec<EventUserResult>,
    #[serde(default)]
    pub callback_url: Option<String>,
    #[serde(default)]
    #[serde(alias = "options")]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub request_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEnvelope {
    pub kind: String,
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub notification_delivery: Option<NotificationDelivery>,
    #[serde(default)]
    pub payload: SyncPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSigningKey {
    pub key_id: String,
    #[serde(default = "default_decision_signature_algorithm")]
    pub algorithm: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionSignature {
    pub key_id: String,
    #[serde(default = "default_decision_signature_algorithm")]
    pub algorithm: String,
    pub nonce: String,
    pub signed_at: String,
    pub request_digest: String,
    pub signature: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncPayload {
    #[serde(default, alias = "request")]
    pub event: Option<Event>,
    #[serde(default, alias = "source")]
    pub channel: Option<Channel>,
    #[serde(default)]
    pub notification_delivery: Option<NotificationDelivery>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientState {
    pub servers: Vec<ServerProfile>,
    pub selected_server_id: Option<String>,
    pub current_user: Option<User>,
    pub devices: Vec<UserDevice>,
    pub channels: Vec<Channel>,
    pub pending_counts_by_channel: BTreeMap<String, usize>,
    pub events: Vec<Event>,
    pub selected_channel_id: Option<String>,
    pub selected_event_id: Option<String>,
    pub notification_sound: String,
    #[serde(default)]
    pub notification_delivery_mode: NotificationDeliveryMode,
    pub is_registered: bool,
    pub is_sync_connected: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollDeviceResponse {
    pub device_id: String,
    pub token: String,
    #[serde(alias = "sources")]
    pub channels: Vec<Channel>,
    #[serde(default = "default_user_id")]
    pub user_id: String,
    #[serde(default = "default_user_name")]
    pub user_name: String,
    #[serde(default)]
    pub devices: Vec<UserDevice>,
    #[serde(default)]
    pub notification_delivery: NotificationDelivery,
}

#[derive(Debug, Deserialize)]
pub struct CurrentUserResponse {
    pub user: User,
    pub current_device: UserDevice,
    #[serde(default)]
    pub notification_delivery: NotificationDelivery,
}

#[derive(Debug, Deserialize)]
pub struct UserDevicesResponse {
    pub devices: Vec<UserDevice>,
}

#[derive(Debug, Deserialize)]
pub struct UserDeviceResponse {
    pub device: UserDevice,
}

#[derive(Debug, Deserialize)]
pub struct ChannelsResponse {
    #[serde(alias = "sources")]
    pub channels: Vec<Channel>,
}

#[derive(Debug, Deserialize)]
pub struct EventsResponse {
    #[serde(alias = "requests")]
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
pub struct EventResponse {
    #[serde(alias = "request")]
    pub event: Event,
}

fn default_true() -> bool {
    true
}

fn default_style() -> String {
    "default".to_string()
}

fn default_decision_signature_algorithm() -> String {
    crate::signing::DECISION_SIGNING_ALGORITHM.to_string()
}

fn default_action_resolution() -> ActionResolution {
    ActionResolution::Shared
}

fn default_user_id() -> String {
    "default".to_string()
}

fn default_user_name() -> String {
    "Nod".to_string()
}
