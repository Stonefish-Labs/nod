use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAttestationSummary {
    pub provider: String,
    pub status: DeviceAttestationStatus,
    pub key_id: Option<String>,
    pub team_id: Option<String>,
    pub bundle_id: Option<String>,
    pub environment: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceAttestationStatus {
    Verified,
    Failed,
}

impl DeviceAttestationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Failed => "failed",
        }
    }
}

impl From<&str> for DeviceAttestationStatus {
    fn from(value: &str) -> Self {
        match value {
            "verified" => Self::Verified,
            _ => Self::Failed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceAttestationRecord {
    pub provider: String,
    pub status: DeviceAttestationStatus,
    pub key_id: Option<String>,
    pub team_id: Option<String>,
    pub bundle_id: Option<String>,
    pub environment: Option<String>,
    pub public_key: Option<String>,
    pub counter: Option<i64>,
    pub receipt_hash: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifiedDeviceAttestation {
    pub provider: String,
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub environment: String,
    pub public_key: String,
    pub counter: i64,
    pub receipt_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FailedDeviceAttestation {
    pub provider: String,
    pub key_id: Option<String>,
    pub team_id: Option<String>,
    pub environment: Option<String>,
    pub reason: String,
}

impl DeviceAttestationRecord {
    pub fn verified(details: VerifiedDeviceAttestation) -> Self {
        Self {
            provider: details.provider,
            status: DeviceAttestationStatus::Verified,
            key_id: Some(details.key_id),
            team_id: Some(details.team_id),
            bundle_id: Some(details.bundle_id),
            environment: Some(details.environment),
            public_key: Some(details.public_key),
            counter: Some(details.counter),
            receipt_hash: details.receipt_hash,
            verified_at: Some(Utc::now()),
            failure_reason: None,
        }
    }

    pub fn failed(details: FailedDeviceAttestation) -> Self {
        Self {
            provider: details.provider,
            status: DeviceAttestationStatus::Failed,
            key_id: details.key_id,
            team_id: details.team_id,
            bundle_id: None,
            environment: details.environment,
            public_key: None,
            counter: None,
            receipt_hash: None,
            verified_at: None,
            failure_reason: Some(sanitize_attestation_failure(&details.reason)),
        }
    }
}

fn sanitize_attestation_failure(reason: &str) -> String {
    const MAX_REASON_LEN: usize = 160;
    let reason = reason.trim();
    if reason.len() <= MAX_REASON_LEN {
        return reason.to_string();
    }
    reason.chars().take(MAX_REASON_LEN).collect()
}
