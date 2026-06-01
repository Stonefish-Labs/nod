use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{SecondsFormat, Utc};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::{Action, ActionKind, DecisionSignature, DeviceSigningKey, Event};

pub const DECISION_SIGNING_ALGORITHM: &str = "p256_ecdsa_sha256";

const IMPLICIT_DISMISS_ACTION_ID: &str = "dismiss";
const SIGNING_PAYLOAD_VERSION: &str = "nod-decision-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSigningKey {
    pub key_id: String,
    pub private_key: String,
}

impl StoredSigningKey {
    pub fn generate() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        Self {
            key_id: uuid::Uuid::new_v4().to_string(),
            private_key: URL_SAFE_NO_PAD.encode(signing_key.to_bytes()),
        }
    }

    pub fn device_signing_key(&self) -> Result<DeviceSigningKey> {
        let signing_key = self.signing_key()?;
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        Ok(DeviceSigningKey {
            key_id: self.key_id.clone(),
            algorithm: DECISION_SIGNING_ALGORITHM.to_string(),
            public_key: URL_SAFE_NO_PAD.encode(public_key),
        })
    }

    pub fn sign_decision(&self, request: DecisionSigningRequest<'_>) -> Result<DecisionSignature> {
        let request_digest = request
            .event
            .request_digest
            .as_deref()
            .ok_or_else(|| anyhow!("request digest is missing for {}", request.event.id))?;
        let action = action_for(request.event, request.action_id)?;
        let nonce = uuid::Uuid::new_v4().to_string();
        let signed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let text = request.text.and_then(trimmed_text);
        let payload = decision_signing_payload(DecisionSigningPayload {
            event: request.event,
            action,
            text,
            user_id: request.user_id,
            device_id: request.device_id,
            key_id: &self.key_id,
            nonce: &nonce,
            signed_at: &signed_at,
            request_digest,
        });
        let signature: Signature = self.signing_key()?.sign(payload.as_bytes());
        Ok(DecisionSignature {
            key_id: self.key_id.clone(),
            algorithm: DECISION_SIGNING_ALGORITHM.to_string(),
            nonce,
            signed_at,
            request_digest: request_digest.to_string(),
            signature: URL_SAFE_NO_PAD.encode(signature.to_der().as_bytes()),
        })
    }

    fn signing_key(&self) -> Result<SigningKey> {
        let private_key = URL_SAFE_NO_PAD.decode(&self.private_key)?;
        SigningKey::from_slice(&private_key).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DecisionSigningRequest<'a> {
    pub event: &'a Event,
    pub action_id: &'a str,
    pub text: Option<&'a str>,
    pub user_id: &'a str,
    pub device_id: &'a str,
}

struct DecisionSigningPayload<'a> {
    event: &'a Event,
    action: ActionRef<'a>,
    text: Option<&'a str>,
    user_id: &'a str,
    device_id: &'a str,
    key_id: &'a str,
    nonce: &'a str,
    signed_at: &'a str,
    request_digest: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct ActionRef<'a> {
    id: &'a str,
    kind: &'a ActionKind,
}

impl<'a> From<&'a Action> for ActionRef<'a> {
    fn from(action: &'a Action) -> Self {
        Self {
            id: &action.id,
            kind: &action.kind,
        }
    }
}

fn action_for<'a>(event: &'a Event, action_id: &'a str) -> Result<ActionRef<'a>> {
    if let Some(action) = event.actions.iter().find(|action| action.id == action_id) {
        return Ok(action.into());
    }

    // The UI can synthesize a dismiss button for actionless events. The server
    // still verifies that submission against the same canonical payload shape.
    if action_id == IMPLICIT_DISMISS_ACTION_ID {
        return Ok(ActionRef {
            id: IMPLICIT_DISMISS_ACTION_ID,
            kind: &ActionKind::Dismiss,
        });
    }
    Err(anyhow!(
        "action {action_id} is not available for {}",
        event.id
    ))
}

fn decision_signing_payload(payload: DecisionSigningPayload<'_>) -> String {
    // The server verifies this canonical payload. Field names, order, and the
    // trailing newline are part of the signing contract.
    [
        SIGNING_PAYLOAD_VERSION.to_string(),
        format!("request_id:{}", payload.event.id),
        format!("request_digest:{}", payload.request_digest),
        format!("option_id:{}", payload.action.id),
        format!("option_kind:{}", payload.action.kind.as_str()),
        format!("user_id:{}", payload.user_id),
        format!("device_id:{}", payload.device_id),
        format!("key_id:{}", payload.key_id),
        format!("nonce:{}", payload.nonce),
        format!("signed_at:{}", payload.signed_at),
        format!("text_sha256:{}", sha256_hex(payload.text.unwrap_or(""))),
        String::new(),
    ]
    .join("\n")
}

fn trimmed_text(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn sha256_hex(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::models::{ActionResolution, EventStatus};

    const UNCOMPRESSED_X963_PUBLIC_KEY_LENGTH: usize = 65;
    const UNCOMPRESSED_X963_PUBLIC_KEY_PREFIX: u8 = 4;

    fn event() -> Event {
        Event {
            id: "request-1".to_string(),
            channel_id: "default".to_string(),
            recipients: Vec::new(),
            action_resolution: ActionResolution::Shared,
            title: "Deploy".to_string(),
            summary: String::new(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            priority: 5,
            privacy: "private".to_string(),
            dedupe_key: None,
            expires_at: None,
            status: EventStatus::Pending,
            created_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            resolved_at: None,
            result: None,
            user_results: Vec::new(),
            callback_url: None,
            actions: vec![Action {
                id: "approve".to_string(),
                label: "Approve".to_string(),
                kind: ActionKind::Approve,
                style: "default".to_string(),
                requires_text: false,
                text_placeholder: None,
                destructive: false,
                foreground: false,
            }],
            request_digest: Some("server-provided-request-digest".to_string()),
        }
    }

    #[test]
    fn decision_payload_matches_server_contract() {
        let event = event();
        let payload = decision_signing_payload(DecisionSigningPayload {
            event: &event,
            action: (&event.actions[0]).into(),
            text: Some("ship it"),
            user_id: "user-1",
            device_id: "device-1",
            key_id: "device-key-id",
            nonce: "unique-device-nonce",
            signed_at: "2026-05-31T12:00:00.000Z",
            request_digest: "server-provided-request-digest",
        });

        assert_eq!(
            payload,
            concat!(
                "nod-decision-v1\n",
                "request_id:request-1\n",
                "request_digest:server-provided-request-digest\n",
                "option_id:approve\n",
                "option_kind:approve\n",
                "user_id:user-1\n",
                "device_id:device-1\n",
                "key_id:device-key-id\n",
                "nonce:unique-device-nonce\n",
                "signed_at:2026-05-31T12:00:00.000Z\n",
                "text_sha256:bef4261f394bf71fd2b565cd76396ac9ed7953f9110c69ee49d7a82871238fbf\n"
            )
        );
    }

    #[test]
    fn actionless_events_can_still_dismiss() {
        let mut event = event();
        event.actions.clear();

        let action = action_for(&event, IMPLICIT_DISMISS_ACTION_ID)
            .expect("implicit dismiss should be available");

        assert_eq!(action.id, IMPLICIT_DISMISS_ACTION_ID);
        assert_eq!(*action.kind, ActionKind::Dismiss);
    }

    #[test]
    fn blank_decision_text_is_signed_as_empty() {
        assert_eq!(trimmed_text("  \n\t  "), None);
    }

    #[test]
    fn signing_fails_when_request_digest_is_missing() {
        let mut event = event();
        event.request_digest = None;
        let key = StoredSigningKey::generate();
        let error = key
            .sign_decision(DecisionSigningRequest {
                event: &event,
                action_id: "approve",
                text: None,
                user_id: "user-1",
                device_id: "device-1",
            })
            .expect_err("missing digest should fail signing");

        assert!(error.to_string().contains("request digest is missing"));
    }

    #[test]
    fn signing_fails_when_action_is_not_available() {
        let event = event();
        let key = StoredSigningKey::generate();
        let error = key
            .sign_decision(DecisionSigningRequest {
                event: &event,
                action_id: "reject",
                text: None,
                user_id: "user-1",
                device_id: "device-1",
            })
            .expect_err("unknown action should fail signing");

        assert!(error.to_string().contains("action reject is not available"));
    }

    #[test]
    fn generated_public_key_uses_uncompressed_x963_bytes() {
        let key = StoredSigningKey::generate();
        let public_key = key
            .device_signing_key()
            .expect("generated key should expose a public key");
        let bytes = URL_SAFE_NO_PAD
            .decode(public_key.public_key)
            .expect("public key should be base64url encoded");

        assert_eq!(bytes.len(), UNCOMPRESSED_X963_PUBLIC_KEY_LENGTH);
        assert_eq!(bytes[0], UNCOMPRESSED_X963_PUBLIC_KEY_PREFIX);
    }
}
