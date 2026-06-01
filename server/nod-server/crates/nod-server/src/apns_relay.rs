use std::{fs, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Certificate, Client, Identity};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    config::ApnsRelayConfig,
    models::{DecisionRequest, Device},
    push::{PushCategory, PushProvider, APPLE_APNS_PROVIDER_ID},
};

const APNS_RELAY_PUSH_PATH: &str = "/v1/notifications";

pub struct ApnsRelayProvider {
    client: Client,
    url: String,
    native_app_id: String,
}

impl ApnsRelayProvider {
    pub fn new(config: ApnsRelayConfig) -> anyhow::Result<Self> {
        let raw_url = config
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("missing APNs relay URL"))?;
        let url = Url::parse(raw_url)?;
        if url.scheme() != "https" {
            anyhow::bail!("APNs relay URL must use https");
        }
        let native_app_id = required_native_app_id(config.native_app_id.as_deref())?;
        Ok(Self {
            client: relay_client(&config)?,
            url: url.as_str().trim_end_matches('/').to_string(),
            native_app_id,
        })
    }
}

#[async_trait]
impl PushProvider for ApnsRelayProvider {
    fn id(&self) -> &str {
        APPLE_APNS_PROVIDER_ID
    }

    fn native_app_id(&self) -> Option<&str> {
        Some(&self.native_app_id)
    }

    async fn push_request(&self, device: &Device, request: &DecisionRequest) -> anyhow::Result<()> {
        let Some(relay_request) = ApnsRelayRequest::from_device_request(device, request) else {
            return Ok(());
        };
        let response = self
            .client
            .post(format!("{}{}", self.url, APNS_RELAY_PUSH_PATH))
            .json(&relay_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("APNs relay rejected push with {status}: {text}");
        }
        tracing::info!(device_id = %device.id, request_id = %request.id, "push delivered to APNs relay");
        Ok(())
    }
}

fn relay_client(config: &ApnsRelayConfig) -> anyhow::Result<Client> {
    let tls = &config.tls;
    let client_cert_path = tls
        .client_cert_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing APNs relay client certificate path"))?;
    let client_key_path = tls
        .client_key_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing APNs relay client key path"))?;
    let ca_cert_path = tls
        .ca_cert_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing APNs relay CA certificate path"))?;

    let mut identity_pem = fs::read(client_cert_path).with_context(|| {
        format!(
            "failed to read APNs relay client certificate {}",
            client_cert_path.display()
        )
    })?;
    identity_pem.extend(fs::read(client_key_path).with_context(|| {
        format!(
            "failed to read APNs relay client key {}",
            client_key_path.display()
        )
    })?);
    let identity = Identity::from_pem(&identity_pem)?;
    let ca_pem = fs::read(ca_cert_path).with_context(|| {
        format!(
            "failed to read APNs relay CA certificate {}",
            ca_cert_path.display()
        )
    })?;
    let certificates = Certificate::from_pem_bundle(&ca_pem)?;
    if certificates.is_empty() {
        anyhow::bail!("APNs relay CA bundle did not contain certificates");
    }

    let mut builder = Client::builder()
        .https_only(true)
        .identity(identity)
        .timeout(Duration::from_secs(10));
    for certificate in certificates {
        builder = builder.add_root_certificate(certificate);
    }
    Ok(builder.build()?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApnsRelayRequest {
    pub target: NotificationTarget,
    pub notification: RelayNotification,
    pub metadata: RelayMetadata,
}

impl ApnsRelayRequest {
    pub fn from_device_request(device: &Device, request: &DecisionRequest) -> Option<Self> {
        device
            .push_provider
            .as_deref()
            .map(str::trim)
            .filter(|provider| *provider == APPLE_APNS_PROVIDER_ID)?;
        let token = device.push_token.as_deref()?.trim();
        if token.is_empty() {
            return None;
        }
        Some(Self {
            target: NotificationTarget {
                platform: device.platform.as_str().to_string(),
                native_app_id: device.native_app_id.as_ref()?.to_string(),
                token: token.to_string(),
            },
            notification: RelayNotification {
                title: request.title.clone(),
                body: request.summary.clone(),
                sound: device.notification_sound.clone(),
                thread_id: request.source_id.clone(),
                category: PushCategory::for_request(request).as_str().to_string(),
            },
            metadata: RelayMetadata {
                request_id: request.id.clone(),
                source_id: request.source_id.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationTarget {
    pub platform: String,
    pub native_app_id: String,
    pub token: String,
}

fn required_native_app_id(value: Option<&str>) -> anyhow::Result<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("missing APNs relay native app id"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayNotification {
    pub title: String,
    pub body: String,
    pub sound: String,
    pub thread_id: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayMetadata {
    pub request_id: String,
    pub source_id: String,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::config::{ApnsRelayConfig, ApnsRelayTlsConfig};
    use crate::models::{DecisionResolution, DevicePlatform, RequestStatus};

    use super::*;

    #[test]
    fn apns_relay_request_serializes_apns_relay_contract() {
        let now = Utc::now();
        let device = Device {
            id: "device-1".to_string(),
            user_id: "owner".to_string(),
            name: "Phone".to_string(),
            platform: DevicePlatform::Ios,
            native_app_id: Some("com.example.NodTests".to_string()),
            push_provider: Some("apple_apns".to_string()),
            push_token: Some("push-token".to_string()),
            signing_key_id: None,
            signing_key_algorithm: None,
            signing_public_key: None,
            notification_sound: "default".to_string(),
            last_seen_at: now,
            created_at: now,
        };
        let request = DecisionRequest {
            id: "request-1".to_string(),
            source_id: "default".to_string(),
            recipients: vec!["owner".to_string()],
            decision_resolution: DecisionResolution::Shared,
            title: "Deploy".to_string(),
            summary: "Production deploy is waiting".to_string(),
            body_markdown: "secret details".to_string(),
            fields: vec![],
            links: vec![],
            image_url: None,
            priority: 5,
            privacy: "private".to_string(),
            dedupe_key: None,
            expires_at: None,
            status: RequestStatus::Pending,
            created_at: now,
            updated_at: now,
            resolved_at: None,
            decision: None,
            user_decisions: vec![],
            callback_url: None,
            options: vec![],
        };

        let request = ApnsRelayRequest::from_device_request(&device, &request).unwrap();
        let value = serde_json::to_value(request).unwrap();

        assert!(value.get("provider").is_none());
        assert_eq!(value["target"]["platform"], "ios");
        assert_eq!(value["target"]["native_app_id"], "com.example.NodTests");
        assert_eq!(value["target"]["token"], "push-token");
        assert_eq!(value["notification"]["title"], "Deploy");
        assert_eq!(
            value["notification"]["body"],
            "Production deploy is waiting"
        );
        assert_eq!(value["notification"]["category"], "NOD_DEFAULT");
        assert_eq!(value["metadata"]["request_id"], "request-1");
        assert!(value["body_markdown"].is_null());
    }

    #[test]
    fn apns_relay_provider_loads_mtls_identity() {
        let config = ApnsRelayConfig {
            url: Some("https://relay.example.com".to_string()),
            native_app_id: Some("com.example.NodTests".to_string()),
            tls: ApnsRelayTlsConfig {
                client_cert_path: Some("tests/fixtures/relay-tls/client.crt".into()),
                client_key_path: Some("tests/fixtures/relay-tls/client.key".into()),
                ca_cert_path: Some("tests/fixtures/relay-tls/server-ca.crt".into()),
            },
        };

        ApnsRelayProvider::new(config).unwrap();
    }
}
