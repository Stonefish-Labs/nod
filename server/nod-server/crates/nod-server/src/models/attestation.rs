use chrono::{DateTime, Utc};

// The public attestation wire types live in nod-proto (single source of truth)
// so the Rust clients and generated Swift/TS describe attestation identically.
// The server-only record / verification types stay here.
pub use nod_proto::{DeviceAttestationStatus, DeviceAttestationSummary};

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
