use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AdminCounts {
    pub users: i64,
    pub sources: i64,
    pub devices: i64,
    pub active_issuer_tokens: i64,
    pub pending_requests: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminSummary {
    pub users: i64,
    pub sources: i64,
    pub devices: i64,
    pub active_issuer_tokens: i64,
    pub pending_requests: i64,
    pub notification_delivery_mode: String,
    pub remote_push_route: Option<String>,
    pub retention_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminSettings {
    pub notification_delivery_mode: String,
    pub remote_push_route: Option<String>,
    pub retention_days: i64,
    pub apns_relay: AdminApnsRelaySettings,
    pub device_attestation: AdminDeviceAttestationSettings,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminDeviceAttestationSettings {
    pub apple_app_attest: AdminAppleAppAttestSettings,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminAppleAppAttestSettings {
    pub mode: String,
    pub team_id_configured: bool,
    pub bundle_ids: Vec<String>,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminApnsRelaySettings {
    pub client_enabled: bool,
    pub url: Option<String>,
    pub native_app_id: Option<String>,
    pub client_cert_configured: bool,
    pub client_key_configured: bool,
    pub ca_cert_configured: bool,
}
