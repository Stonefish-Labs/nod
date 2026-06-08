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
pub struct Source {
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
pub enum OptionKind {
    Approve,
    ApproveWithText,
    Reject,
    RejectWithText,
    Dismiss,
    Open,
    Custom,
}

impl OptionKind {
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
pub struct RequestOption {
    pub id: String,
    pub label: String,
    pub kind: OptionKind,
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
pub enum RequestStatus {
    Pending,
    Resolved,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Decision {
    pub request_id: String,
    pub option_id: String,
    pub option_kind: OptionKind,
    pub option_label: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub actor_user_id: Option<String>,
    #[serde(default)]
    pub actor_device_id: Option<String>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserDecision {
    pub user_id: String,
    pub decision: Decision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionResolution {
    Shared,
    PerUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub request_id: String,
    pub source_id: String,
    #[serde(default)]
    pub recipients: Vec<String>,
    #[serde(default = "default_decision_resolution")]
    pub decision_resolution: DecisionResolution,
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
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub decision: Option<Decision>,
    #[serde(default)]
    pub decisions: Vec<UserDecision>,
    #[serde(default)]
    pub callback_url: Option<String>,
    pub options: Vec<RequestOption>,
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
    #[serde(default)]
    pub request: Option<Request>,
    #[serde(default)]
    pub source: Option<Source>,
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
    pub sources: Vec<Source>,
    pub pending_counts_by_source: BTreeMap<String, usize>,
    pub requests: Vec<Request>,
    pub selected_source_id: Option<String>,
    pub selected_request_id: Option<String>,
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
    pub sources: Vec<Source>,
    pub user_id: String,
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
pub struct SourcesResponse {
    pub sources: Vec<Source>,
}

#[derive(Debug, Deserialize)]
pub struct RequestsResponse {
    pub requests: Vec<Request>,
}

#[derive(Debug, Deserialize)]
pub struct RequestResponse {
    pub request: Request,
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

fn default_decision_resolution() -> DecisionResolution {
    DecisionResolution::Shared
}
