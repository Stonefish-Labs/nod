//! Device attestation wire types (App Attest / hardware attestation summary).
//!
//! Shared so the server, the Rust clients (`nod-client-core`), and the generated
//! Swift/TS types all describe a device's attestation the same way — the server
//! was the only owner, which silently dropped these fields on the client side.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/// The public attestation summary the server reports for a device.
#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceAttestationSummary {
    pub provider: String,
    pub status: DeviceAttestationStatus,
    pub key_id: Option<String>,
    pub team_id: Option<String>,
    pub bundle_id: Option<String>,
    pub environment: Option<String>,
    #[typeshare(serialized_as = "Option<String>")]
    pub verified_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

/// Whether a device's attestation verified or failed.
#[typeshare]
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
