use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::ApnsConfig,
    relay::{ApnsDelivery, RelayNotification},
};

use super::{auth::ApnsTokenSigner, payload::apns_payload};

// Even when `sound` is omitted, these payloads are alert notifications, not
// background pushes.
const PUSH_TYPE_ALERT: &str = "alert";
const ALERT_PRIORITY: &str = "10";

/// APNs delivery backend that signs each request with provider credentials.
pub struct AppleApnsProvider {
    client: Client,
    bundle_id: String,
    endpoint: String,
    token_signer: ApnsTokenSigner,
}

impl AppleApnsProvider {
    pub fn new(config: ApnsConfig) -> anyhow::Result<Self> {
        let endpoint = config.environment.endpoint().to_string();
        Self::with_endpoint(config, endpoint)
    }

    pub(crate) fn with_endpoint(config: ApnsConfig, endpoint: String) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder().http2_adaptive_window(true).build()?,
            bundle_id: config.bundle_id,
            endpoint,
            token_signer: ApnsTokenSigner::from_credentials(config.credentials)?,
        })
    }
}

#[async_trait]
impl ApnsDelivery for AppleApnsProvider {
    async fn send(&self, notification: &RelayNotification) -> anyhow::Result<()> {
        let url = format!("{}/3/device/{}", self.endpoint, notification.target.token);
        let response = self
            .client
            .post(url)
            .bearer_auth(self.token_signer.jwt()?)
            .header("apns-topic", &self.bundle_id)
            .header("apns-push-type", PUSH_TYPE_ALERT)
            .header("apns-priority", ALERT_PRIORITY)
            .json(&apns_payload(notification))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Apple APNs rejected push with {status}: {text}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use axum::{http::StatusCode, routing::post, Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    use crate::{
        config::{ApnsConfig, ApnsCredentials, ApnsEnvironment},
        relay::{
            RelayNotification, RelayNotificationContent, RelayNotificationMetadata, RelayTarget,
            TargetPlatform,
        },
    };

    use super::*;

    #[tokio::test]
    async fn apns_provider_reports_upstream_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/3/device/device-token",
            post(|| async {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"reason": "nope"})),
                )
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let provider =
            AppleApnsProvider::with_endpoint(test_config(), format!("http://{addr}")).unwrap();

        let err = provider
            .send(&valid_notification())
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("Apple APNs rejected push"));
    }

    fn test_config() -> ApnsConfig {
        ApnsConfig {
            bundle_id: "com.example.NodTests".to_string(),
            credentials: ApnsCredentials {
                team_id: "TEAMID".to_string(),
                key_id: "KEYID".to_string(),
                private_key_path: "tests/fixtures/mtls/apns-auth-key.p8".into(),
            },
            environment: ApnsEnvironment::Sandbox,
        }
    }

    fn valid_notification() -> RelayNotification {
        RelayNotification {
            target: RelayTarget {
                platform: TargetPlatform::Ios,
                native_app_id: "com.example.NodTests".to_string(),
                token: "device-token".to_string(),
            },
            notification: RelayNotificationContent {
                title: "Deploy".to_string(),
                body: "Production deploy is waiting".to_string(),
                sound: "nod_ping.wav".to_string(),
                thread_id: "default".to_string(),
                category: "NOD_APPROVAL".to_string(),
            },
            metadata: RelayNotificationMetadata {
                request_id: "request-1".to_string(),
                source_id: "default".to_string(),
            },
        }
    }
}
