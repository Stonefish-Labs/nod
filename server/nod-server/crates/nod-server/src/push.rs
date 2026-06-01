use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    apns_relay,
    config::NotificationsConfig,
    models::{DecisionRequest, Device, NotificationDelivery, NotificationDeliveryMode},
};

pub const APPLE_APNS_PROVIDER_ID: &str = "apple_apns";

#[async_trait]
pub trait PushProvider: Send + Sync {
    fn id(&self) -> &str;

    fn native_app_id(&self) -> Option<&str> {
        None
    }

    async fn push_request(&self, device: &Device, request: &DecisionRequest) -> anyhow::Result<()>;
}

pub type DynPushProvider = Arc<dyn PushProvider>;

#[derive(Clone, Default)]
pub struct PushRegistry {
    providers_by_route: Arc<HashMap<PushRouteKey, DynPushProvider>>,
}

impl PushRegistry {
    pub fn new(providers: Vec<DynPushProvider>) -> Self {
        let mut providers_by_route = HashMap::new();
        for provider in providers {
            let route = PushRouteKey::new(provider.id(), provider.native_app_id());
            providers_by_route.insert(route, provider);
        }
        Self {
            providers_by_route: Arc::new(providers_by_route),
        }
    }

    pub async fn push_request(
        &self,
        device: &Device,
        request: &DecisionRequest,
    ) -> anyhow::Result<()> {
        let Some(route) = PushRouteKey::from_device(device) else {
            tracing::debug!(device_id = %device.id, request_id = %request.id, "push skipped because device has no provider");
            return Ok(());
        };
        let Some(provider) = self.providers_by_route.get(&route) else {
            tracing::debug!(
                device_id = %device.id,
                request_id = %request.id,
                provider_id = %route.provider_id,
                native_app_id = route.native_app_id.as_deref(),
                "push skipped because provider route is not configured"
            );
            return Ok(());
        };
        provider.push_request(device, request).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PushRouteKey {
    provider_id: String,
    native_app_id: Option<String>,
}

impl PushRouteKey {
    fn new(provider_id: &str, native_app_id: Option<&str>) -> Self {
        Self {
            provider_id: provider_id.trim().to_string(),
            native_app_id: native_app_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        }
    }

    fn from_device(device: &Device) -> Option<Self> {
        let provider_id = device
            .push_provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        Some(Self::new(provider_id, device.native_app_id.as_deref()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PushCategory {
    #[serde(rename = "NOD_DEFAULT")]
    Default,
    #[serde(rename = "NOD_APPROVAL")]
    Approval,
    #[serde(rename = "NOD_APPROVAL_TEXT")]
    ApprovalText,
}

impl PushCategory {
    pub fn for_request(request: &DecisionRequest) -> Self {
        if request.options.iter().any(|option| option.requires_text) {
            Self::ApprovalText
        } else if request.options.is_empty() {
            Self::Default
        } else {
            Self::Approval
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "NOD_DEFAULT",
            Self::Approval => "NOD_APPROVAL",
            Self::ApprovalText => "NOD_APPROVAL_TEXT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePushRoute {
    ApnsRelay,
}

impl RemotePushRoute {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApnsRelay => "apns_relay",
        }
    }
}

#[derive(Clone)]
pub struct BuiltPushRegistry {
    pub registry: PushRegistry,
    pub remote_route: Option<RemotePushRoute>,
}

pub fn notification_delivery_for_route(
    remote_route: Option<RemotePushRoute>,
) -> NotificationDelivery {
    NotificationDelivery {
        mode: if remote_route.is_some() {
            NotificationDeliveryMode::Push
        } else {
            NotificationDeliveryMode::Websocket
        },
    }
}

pub fn configured_remote_push_route(config: &NotificationsConfig) -> Option<RemotePushRoute> {
    if config.apns_relay.client_enabled() {
        Some(RemotePushRoute::ApnsRelay)
    } else {
        None
    }
}

pub fn build_push_registry(config: &NotificationsConfig) -> anyhow::Result<BuiltPushRegistry> {
    match configured_remote_push_route(config) {
        Some(RemotePushRoute::ApnsRelay) => {
            let provider = apns_relay::ApnsRelayProvider::new(config.apns_relay.clone())?;
            Ok(BuiltPushRegistry {
                registry: PushRegistry::new(vec![Arc::new(provider)]),
                remote_route: Some(RemotePushRoute::ApnsRelay),
            })
        }
        None => Ok(BuiltPushRegistry {
            registry: PushRegistry::default(),
            remote_route: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::config::{ApnsRelayConfig, ApnsRelayTlsConfig, NotificationsConfig};

    use super::*;

    struct NoopProvider {
        native_app_id: Option<&'static str>,
    }

    #[async_trait]
    impl PushProvider for NoopProvider {
        fn id(&self) -> &str {
            APPLE_APNS_PROVIDER_ID
        }

        fn native_app_id(&self) -> Option<&str> {
            self.native_app_id
        }

        async fn push_request(
            &self,
            _device: &Device,
            _request: &DecisionRequest,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn registry_routes_are_scoped_by_native_app_id() {
        let registry = PushRegistry::new(vec![Arc::new(NoopProvider {
            native_app_id: Some("com.example.NodTests"),
        })]);

        assert!(registry.providers_by_route.contains_key(&PushRouteKey::new(
            APPLE_APNS_PROVIDER_ID,
            Some("com.example.NodTests")
        )));
        assert!(!registry
            .providers_by_route
            .contains_key(&PushRouteKey::new(APPLE_APNS_PROVIDER_ID, None)));
    }

    #[test]
    fn apns_relay_is_the_only_remote_push_route() {
        let config = NotificationsConfig {
            apns_relay: ApnsRelayConfig {
                url: Some("https://relay.example.com".to_string()),
                native_app_id: Some("com.example.NodTests".to_string()),
                tls: ApnsRelayTlsConfig {
                    client_cert_path: Some("client.crt".into()),
                    client_key_path: Some("client.key".into()),
                    ca_cert_path: Some("ca.crt".into()),
                },
            },
        };
        assert_eq!(
            configured_remote_push_route(&config),
            Some(RemotePushRoute::ApnsRelay)
        );
    }

    #[test]
    fn websocket_selected_without_configured_provider() {
        assert_eq!(
            configured_remote_push_route(&NotificationsConfig::default()),
            None
        );
    }

    #[test]
    fn build_fails_when_apns_relay_cert_files_are_missing() {
        let config = NotificationsConfig {
            apns_relay: ApnsRelayConfig {
                url: Some("https://relay.example.com".to_string()),
                native_app_id: Some("com.example.NodTests".to_string()),
                tls: ApnsRelayTlsConfig {
                    client_cert_path: Some("missing/client.crt".into()),
                    client_key_path: Some("missing/client.key".into()),
                    ca_cert_path: Some("missing/ca.crt".into()),
                },
            },
        };

        let err = build_push_registry(&config).err().unwrap().to_string();

        assert!(err.contains("missing/client.crt"), "{err}");
    }

    #[test]
    fn build_reports_effective_push_for_valid_apns_relay() {
        let config = NotificationsConfig {
            apns_relay: ApnsRelayConfig {
                url: Some("https://relay.example.com".to_string()),
                native_app_id: Some("com.example.NodTests".to_string()),
                tls: ApnsRelayTlsConfig {
                    client_cert_path: Some("tests/fixtures/relay-tls/client.crt".into()),
                    client_key_path: Some("tests/fixtures/relay-tls/client.key".into()),
                    ca_cert_path: Some("tests/fixtures/relay-tls/server-ca.crt".into()),
                },
            },
        };
        let built = build_push_registry(&config).unwrap();
        assert_eq!(built.remote_route, Some(RemotePushRoute::ApnsRelay));
        assert_eq!(
            notification_delivery_for_route(built.remote_route).mode,
            NotificationDeliveryMode::Push
        );
    }
}
