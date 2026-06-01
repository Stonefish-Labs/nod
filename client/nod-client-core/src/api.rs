use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::{Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::models::{
    Channel, ChannelsResponse, CurrentUserResponse, DecisionSignature, DevicePlatform,
    DeviceSigningKey, EnrollDeviceResponse, Event, EventResponse, EventsResponse, UserDevice,
    UserDeviceResponse, UserDevicesResponse,
};

const API_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct NodApi {
    base_url: Url,
    token: Option<String>,
    client: Client,
}

impl NodApi {
    pub fn new(base_url: &str, token: Option<String>) -> Result<Self> {
        Ok(Self {
            base_url: Url::parse(&normalize_base_url(base_url))?,
            token,
            client: Client::builder().timeout(API_REQUEST_TIMEOUT).build()?,
        })
    }

    pub fn websocket_url(&self) -> Result<Url> {
        let token = self
            .token
            .as_deref()
            .ok_or_else(|| anyhow!("device token is missing"))?;
        let mut url = self.base_url.clone();
        url.set_scheme(if url.scheme() == "https" { "wss" } else { "ws" })
            .map_err(|_| anyhow!("could not set websocket scheme"))?;
        url.set_path(&join_path(self.base_url.path(), "/api/v1/sync"));
        url.query_pairs_mut().clear().append_pair("token", token);
        Ok(url)
    }

    pub async fn enroll(&self, request: EnrollDeviceRequest<'_>) -> Result<EnrollDeviceResponse> {
        self.request(RequestSpec::anonymous_json(
            Method::POST,
            "/api/v1/enroll",
            &request,
        ))
        .await
        .map_err(Into::into)
    }

    pub async fn current_user(&self) -> Result<CurrentUserResponse> {
        self.request(RequestSpec::authenticated(Method::GET, "/api/v1/users/me"))
            .await
            .map_err(Into::into)
    }

    pub async fn devices(&self) -> Result<Vec<UserDevice>> {
        let response: UserDevicesResponse = self
            .request(RequestSpec::authenticated(
                Method::GET,
                "/api/v1/users/me/devices",
            ))
            .await?;
        Ok(response.devices)
    }

    pub async fn rename_device(&self, id: &str, name: &str) -> Result<UserDevice> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
        }
        let path = format!("/api/v1/users/me/devices/{id}");
        let response: UserDeviceResponse = self
            .request(RequestSpec::authenticated_json(
                Method::PUT,
                &path,
                &Body { name },
            ))
            .await?;
        Ok(response.device)
    }

    pub async fn revoke_device(&self, id: &str) -> Result<()> {
        let path = format!("/api/v1/users/me/devices/{id}");
        let _: serde_json::Value = self
            .request(RequestSpec::authenticated(Method::DELETE, &path))
            .await?;
        Ok(())
    }

    pub async fn channels(&self) -> Result<Vec<Channel>> {
        let response: ChannelsResponse = self
            .request(RequestSpec::authenticated(Method::GET, "/api/v1/sources"))
            .await?;
        Ok(response.channels)
    }

    pub async fn events(
        &self,
        channel_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        let path = events_path(channel_id, limit);
        let response: EventsResponse = self
            .request(RequestSpec::authenticated(Method::GET, &path))
            .await?;
        Ok(response.events)
    }

    pub async fn submit_action(&self, request: SubmitActionRequest<'_>) -> Result<Event> {
        #[derive(Serialize)]
        struct Body<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            signature: Option<&'a DecisionSignature>,
        }
        let path = format!(
            "/api/v1/requests/{}/options/{}",
            request.event_id, request.action_id
        );
        let response: EventResponse = self
            .request(RequestSpec::authenticated_json(
                Method::POST,
                &path,
                &Body {
                    text: request.text,
                    signature: request.signature,
                },
            ))
            .await?;
        Ok(response.event)
    }

    pub async fn clear_channel(&self, channel_id: &str) -> Result<()> {
        let path = format!("/api/v1/devices/me/sources/{channel_id}/clear");
        let _: serde_json::Value = self
            .request(RequestSpec::authenticated(Method::POST, &path))
            .await?;
        Ok(())
    }

    pub async fn set_subscription(&self, channel_id: &str, subscribed: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            subscribed: bool,
        }
        let path = format!("/api/v1/devices/me/subscriptions/{channel_id}");
        let _: serde_json::Value = self
            .request(RequestSpec::authenticated_json(
                Method::PUT,
                &path,
                &Body { subscribed },
            ))
            .await?;
        Ok(())
    }

    pub async fn set_notification_sound(&self, notification_sound: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> {
            notification_sound: &'a str,
        }
        let _: serde_json::Value = self
            .request(RequestSpec::authenticated_json(
                Method::PUT,
                "/api/v1/devices/me/preferences",
                &Body { notification_sound },
            ))
            .await?;
        Ok(())
    }

    async fn request<Body, Response>(
        &self,
        spec: RequestSpec<'_, Body>,
    ) -> Result<Response, ApiStatusError>
    where
        Body: Serialize + ?Sized,
        Response: DeserializeOwned,
    {
        let url = self.url(spec.path).map_err(ApiStatusError::other)?;
        let mut request = self.client.request(spec.method, url);
        if let Some(body) = spec.body {
            request = request.json(body);
        }
        if spec.auth == RequestAuth::DeviceToken {
            let token = self
                .token
                .as_deref()
                .ok_or_else(|| ApiStatusError::other(anyhow!("device token is missing")))?;
            request = request.bearer_auth(token);
        }
        let response = request
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(ApiStatusError::other)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(ApiStatusError::other)?;
        if !status.is_success() {
            return Err(ApiStatusError {
                status,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        if bytes.is_empty() {
            return serde_json::from_value(serde_json::Value::Object(Default::default()))
                .map_err(ApiStatusError::other);
        }
        serde_json::from_slice(&bytes).map_err(ApiStatusError::other)
    }

    fn url(&self, path: &str) -> Result<Url> {
        let mut url = self.base_url.clone();
        let (path_part, query_part) = path.split_once('?').unwrap_or((path, ""));
        url.set_path(&join_path(self.base_url.path(), path_part));
        url.set_query(if query_part.is_empty() {
            None
        } else {
            Some(query_part)
        });
        Ok(url)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrollDeviceRequest<'a> {
    pub code: &'a str,
    pub device_name: &'a str,
    pub platform: DevicePlatform,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_key: Option<&'a DeviceSigningKey>,
}

#[derive(Debug, Clone, Copy)]
pub struct SubmitActionRequest<'a> {
    pub event_id: &'a str,
    pub action_id: &'a str,
    pub text: Option<&'a str>,
    pub signature: Option<&'a DecisionSignature>,
}

#[derive(Debug, thiserror::Error)]
#[error("Nod request failed with {status}: {body}")]
pub struct ApiStatusError {
    pub status: StatusCode,
    pub body: String,
}

impl ApiStatusError {
    fn other(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: error.to_string(),
        }
    }
}

#[derive(Debug)]
struct RequestSpec<'a, Body: ?Sized> {
    method: Method,
    path: &'a str,
    body: Option<&'a Body>,
    auth: RequestAuth,
}

impl<'a> RequestSpec<'a, ()> {
    fn authenticated(method: Method, path: &'a str) -> Self {
        Self {
            method,
            path,
            body: None,
            auth: RequestAuth::DeviceToken,
        }
    }
}

impl<'a, Body: ?Sized> RequestSpec<'a, Body> {
    fn anonymous_json(method: Method, path: &'a str, body: &'a Body) -> Self {
        Self {
            method,
            path,
            body: Some(body),
            auth: RequestAuth::Anonymous,
        }
    }

    fn authenticated_json(method: Method, path: &'a str, body: &'a Body) -> Self {
        Self {
            method,
            path,
            body: Some(body),
            auth: RequestAuth::DeviceToken,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestAuth {
    Anonymous,
    DeviceToken,
}

pub fn normalize_base_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.contains("://") {
        return trimmed.to_string();
    }
    if trimmed == "localhost"
        || trimmed.starts_with("localhost:")
        || trimmed.starts_with("127.")
        || trimmed.starts_with("192.168.")
    {
        format!("http://{trimmed}")
    } else {
        format!("https://{trimmed}")
    }
}

pub fn profile_id_for(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url).to_ascii_lowercase();
    let mapped: String = normalized
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let compact = mapped
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        compact.chars().take(80).collect()
    }
}

pub fn display_name_for(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url);
    let Ok(url) = Url::parse(&normalized) else {
        return "Nod Server".to_string();
    };
    let mut name = url.host_str().unwrap_or("Nod Server").to_string();
    let path = url.path().trim_matches('/');
    if !path.is_empty() {
        name.push('/');
        name.push_str(path);
    }
    name
}

fn join_path(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    match (base.is_empty(), path.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{path}"),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}/{path}"),
    }
}

fn events_path(channel_id: Option<&str>, limit: Option<usize>) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("include_cleared", "false");
    if let Some(channel_id) = channel_id {
        query.append_pair("source_id", channel_id);
    }
    if let Some(limit) = limit {
        query.append_pair("limit", &limit.to_string());
    }
    format!("/api/v1/requests?{}", query.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NotificationDeliveryMode, SyncEnvelope};

    #[test]
    fn normalizes_local_and_remote_urls() {
        assert_eq!(
            normalize_base_url("localhost:8767"),
            "http://localhost:8767"
        );
        assert_eq!(
            normalize_base_url("nod.example.com/api/"),
            "https://nod.example.com/api"
        );
    }

    #[test]
    fn builds_profile_ids_from_urls() {
        assert_eq!(
            profile_id_for("https://nod.example.com/api"),
            "https-nod-example-com-api"
        );
    }

    #[test]
    fn builds_events_path_with_encoded_filters() {
        assert_eq!(
            events_path(Some("ops/main channel"), Some(25)),
            "/api/v1/requests?include_cleared=false&source_id=ops%2Fmain+channel&limit=25"
        );
    }

    #[test]
    fn enroll_request_serializes_device_signing_key() {
        let signing_key = DeviceSigningKey {
            key_id: "key-1".to_string(),
            algorithm: "p256_ecdsa_sha256".to_string(),
            public_key: "public-key".to_string(),
        };
        let request = EnrollDeviceRequest {
            code: "ABC123",
            device_name: "Laptop",
            platform: DevicePlatform::Linux,
            signing_key: Some(&signing_key),
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "code": "ABC123",
                "device_name": "Laptop",
                "platform": "linux",
                "signing_key": {
                    "key_id": "key-1",
                    "algorithm": "p256_ecdsa_sha256",
                    "public_key": "public-key"
                }
            })
        );
    }

    #[test]
    fn enrollment_response_accepts_server_sources_field() {
        let response: EnrollDeviceResponse = serde_json::from_value(serde_json::json!({
            "device_id": "device-1",
            "user_id": "owner",
            "user_name": "Owner",
            "token": "device-token",
            "notification_delivery": { "mode": "websocket" },
            "sources": [channel_json("default")],
            "devices": []
        }))
        .unwrap();

        assert_eq!(response.channels[0].id, "default");
    }

    #[test]
    fn channels_response_accepts_server_sources_field() {
        let response: ChannelsResponse = serde_json::from_value(serde_json::json!({
            "sources": [channel_json("default")]
        }))
        .unwrap();

        assert_eq!(response.channels[0].id, "default");
    }

    #[test]
    fn event_accepts_server_per_user_decision_field() {
        let event: Event = serde_json::from_value(event_json("request-1", "resolved")).unwrap();

        assert_eq!(event.user_results[0].user_id, "owner");
        assert_eq!(event.user_results[0].result.action_id, "approve");
    }

    #[test]
    fn sync_payload_accepts_server_request_field() {
        let envelope: SyncEnvelope = serde_json::from_value(serde_json::json!({
            "kind": "created",
            "at": "2026-06-01T00:00:01Z",
            "payload": {
                "request": event_json("request-1", "pending")
            }
        }))
        .unwrap();

        assert_eq!(envelope.payload.event.unwrap().id, "request-1");
    }

    #[test]
    fn sync_payload_accepts_server_source_field() {
        let envelope: SyncEnvelope = serde_json::from_value(serde_json::json!({
            "kind": "source_updated",
            "at": "2026-06-01T00:00:01Z",
            "payload": {
                "source": channel_json("default")
            }
        }))
        .unwrap();

        assert_eq!(envelope.payload.channel.unwrap().id, "default");
    }

    #[test]
    fn sync_payload_accepts_server_hello_notification_delivery() {
        let envelope: SyncEnvelope = serde_json::from_value(serde_json::json!({
            "kind": "hello",
            "at": "2026-06-01T00:00:01Z",
            "payload": {
                "device_id": "device-1",
                "notification_delivery": { "mode": "websocket" }
            }
        }))
        .unwrap();

        assert_eq!(
            envelope.payload.notification_delivery.unwrap().mode,
            NotificationDeliveryMode::Websocket
        );
        assert_eq!(
            envelope.payload.extra["device_id"].as_str(),
            Some("device-1")
        );
    }

    fn channel_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": "Default",
            "icon": "bell",
            "color": "#3B82F6",
            "default_priority": 5,
            "privacy": "private",
            "subscribed": true,
            "created_at": "2026-06-01T00:00:00Z"
        })
    }

    fn event_json(id: &str, status: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "request_id": id,
            "source_id": "default",
            "recipients": ["owner"],
            "decision_resolution": "per_user",
            "title": "Approve deployment",
            "summary": "Deployment is waiting",
            "body_markdown": "",
            "fields": [],
            "links": [],
            "image_url": null,
            "priority": 8,
            "privacy": "private",
            "dedupe_key": null,
            "expires_at": null,
            "status": status,
            "created_at": "2026-06-01T00:00:00Z",
            "updated_at": "2026-06-01T00:00:01Z",
            "resolved_at": "2026-06-01T00:00:02Z",
            "decision": decision_json(id),
            "decisions": [
                {
                    "user_id": "owner",
                    "decision": decision_json(id)
                }
            ],
            "callback_url": null,
            "options": [
                {
                    "id": "approve",
                    "label": "Approve",
                    "kind": "approve"
                }
            ],
            "request_digest": "digest-1"
        })
    }

    fn decision_json(event_id: &str) -> serde_json::Value {
        serde_json::json!({
            "request_id": event_id,
            "option_id": "approve",
            "option_kind": "approve",
            "option_label": "Approve",
            "text": null,
            "actor_user_id": "owner",
            "actor_device_id": "device-1",
            "signature": null,
            "resolved_at": "2026-06-01T00:00:02Z"
        })
    }
}
