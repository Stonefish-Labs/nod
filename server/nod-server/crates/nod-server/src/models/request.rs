use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::default_signature_algorithm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardField {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl From<&str> for OptionKind {
    fn from(value: &str) -> Self {
        match value {
            "approve" => Self::Approve,
            "approve_with_text" => Self::ApproveWithText,
            "reject" => Self::Reject,
            "reject_with_text" => Self::RejectWithText,
            "dismiss" => Self::Dismiss,
            "open" => Self::Open,
            _ => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOption {
    pub id: String,
    pub label: String,
    pub kind: OptionKind,
    #[serde(default = "default_option_style")]
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

fn default_option_style() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Resolved,
    Expired,
    Cancelled,
}

impl From<&str> for RequestStatus {
    fn from(value: &str) -> Self {
        match value {
            "resolved" => Self::Resolved,
            "expired" => Self::Expired,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub request_id: String,
    pub option_id: String,
    pub option_kind: OptionKind,
    pub option_label: String,
    pub text: Option<String>,
    #[serde(default)]
    pub actor_user_id: Option<String>,
    pub actor_device_id: Option<String>,
    #[serde(default)]
    pub signature: Option<DecisionSignatureRecord>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSignatureRecord {
    pub key_id: String,
    pub algorithm: String,
    pub nonce: String,
    pub signed_at: String,
    pub request_digest: String,
    pub signing_payload: String,
    pub signature: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl DecisionResolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::PerUser => "per_user",
        }
    }
}

impl From<&str> for DecisionResolution {
    fn from(value: &str) -> Self {
        match value {
            "per_user" => Self::PerUser,
            _ => Self::Shared,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub id: String,
    pub source_id: String,
    pub recipients: Vec<String>,
    pub decision_resolution: DecisionResolution,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub fields: Vec<CardField>,
    pub links: Vec<CardLink>,
    pub image_url: Option<String>,
    pub priority: i64,
    pub privacy: String,
    pub dedupe_key: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub decision: Option<Decision>,
    #[serde(default)]
    pub user_decisions: Vec<UserDecision>,
    pub callback_url: Option<String>,
    pub options: Vec<RequestOption>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDecisionRequest {
    #[serde(default = "default_source")]
    pub source_id: String,
    #[serde(default)]
    pub recipients: Option<Vec<String>>,
    #[serde(default)]
    pub decision_resolution: Option<DecisionResolution>,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub fields: Vec<CardField>,
    #[serde(default)]
    pub links: Vec<CardLink>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub privacy: Option<String>,
    #[serde(default)]
    pub dedupe_key: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub options: Vec<RequestOption>,
    #[serde(default)]
    pub callback_url: Option<String>,
}

fn default_source() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedDecisionRequest {
    pub request_id: String,
    pub deduped: bool,
    pub request: DecisionRequest,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitDecisionRequest {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub signature: Option<SubmitDecisionSignature>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitDecisionSignature {
    pub key_id: String,
    #[serde(default = "default_signature_algorithm")]
    pub algorithm: String,
    pub nonce: String,
    pub signed_at: String,
    pub request_digest: String,
    pub signature: String,
}
