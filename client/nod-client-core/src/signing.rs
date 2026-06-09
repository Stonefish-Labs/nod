use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{DecisionSignature, DeviceSigningKey, OptionKind, Request, RequestOption};

/// The decision-signing algorithm id, mirrored from nod-proto.
pub const DECISION_SIGNING_ALGORITHM: &str = nod_proto::DECISION_SIGNING_ALGORITHM;

const IMPLICIT_DISMISS_OPTION_ID: &str = "dismiss";

/// A locally stored P-256 signing key (key id + base64url private key). The
/// cryptography lives in `nod-proto`; this just wraps the stored key material
/// and orchestrates option resolution and nonce/timestamp generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSigningKey {
    pub key_id: String,
    pub private_key: String,
}

impl StoredSigningKey {
    pub fn generate() -> Self {
        let keypair = nod_proto::generate_signing_key();
        Self {
            key_id: uuid::Uuid::new_v4().to_string(),
            private_key: keypair.private_key,
        }
    }

    pub fn device_signing_key(&self) -> Result<DeviceSigningKey> {
        let public_key = nod_proto::public_key_for(&self.private_key)?;
        Ok(DeviceSigningKey {
            key_id: self.key_id.clone(),
            algorithm: DECISION_SIGNING_ALGORITHM.to_string(),
            public_key,
        })
    }

    pub fn sign_decision(&self, request: DecisionSigningRequest<'_>) -> Result<DecisionSignature> {
        let request_digest = request
            .request
            .request_digest
            .as_deref()
            .ok_or_else(|| anyhow!("request digest is missing for {}", request.request.id))?;
        // Defense in depth: only sign a request whose digest we can reproduce
        // from the content we received (and rendered to the user), rather than
        // trusting the digest the server asserted.
        let recomputed = nod_proto::request_digest(request.request)?;
        if recomputed != request_digest {
            return Err(anyhow!(
                "request digest does not match request content for {}",
                request.request.id
            ));
        }
        let option = option_for(request.request, request.option_id)?;
        let nonce = uuid::Uuid::new_v4().to_string();
        let signed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let text = request.text.and_then(trimmed_text);
        let payload = nod_proto::decision_signing_payload(nod_proto::DecisionSigningInput {
            request_id: &request.request.id,
            request_digest,
            option_id: option.id,
            option_kind: option.kind,
            user_id: request.user_id,
            device_id: request.device_id,
            key_id: &self.key_id,
            nonce: &nonce,
            signed_at: &signed_at,
            text,
        });
        let signature = nod_proto::sign_payload(&self.private_key, payload.as_bytes())?;
        Ok(DecisionSignature {
            key_id: self.key_id.clone(),
            algorithm: DECISION_SIGNING_ALGORITHM.to_string(),
            nonce,
            signed_at,
            request_digest: request_digest.to_string(),
            signature,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DecisionSigningRequest<'a> {
    pub request: &'a Request,
    pub option_id: &'a str,
    pub text: Option<&'a str>,
    pub user_id: &'a str,
    pub device_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct OptionRef<'a> {
    id: &'a str,
    kind: &'a OptionKind,
}

impl<'a> From<&'a RequestOption> for OptionRef<'a> {
    fn from(option: &'a RequestOption) -> Self {
        Self {
            id: &option.id,
            kind: &option.kind,
        }
    }
}

fn option_for<'a>(request: &'a Request, option_id: &'a str) -> Result<OptionRef<'a>> {
    if let Some(option) = request.options.iter().find(|option| option.id == option_id) {
        return Ok(option.into());
    }

    // The UI can synthesize a dismiss button for optionless requests. The server
    // still verifies that submission against the same canonical payload shape.
    if option_id == IMPLICIT_DISMISS_OPTION_ID {
        return Ok(OptionRef {
            id: IMPLICIT_DISMISS_OPTION_ID,
            kind: &OptionKind::Dismiss,
        });
    }
    Err(anyhow!(
        "option {option_id} is not available for {}",
        request.id
    ))
}

fn trimmed_text(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::models::{DecisionResolution, RequestStatus};

    fn request() -> Request {
        let mut request = Request {
            id: "request-1".to_string(),
            request_id: "request-1".to_string(),
            source_id: "default".to_string(),
            recipients: Vec::new(),
            decision_resolution: DecisionResolution::Shared,
            title: "Deploy".to_string(),
            summary: String::new(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            notification: Default::default(),
            dedupe_key: None,
            expires_at: None,
            status: RequestStatus::Pending,
            created_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap(),
            resolved_at: None,
            decision: None,
            decisions: Vec::new(),
            callback_url: None,
            options: vec![RequestOption {
                id: "approve".to_string(),
                label: "Approve".to_string(),
                kind: OptionKind::Approve,
                style: "default".to_string(),
                requires_text: false,
                text_placeholder: None,
                destructive: false,
                foreground: false,
            }],
            request_digest: None,
        };
        request.request_digest = Some(nod_proto::request_digest(&request).unwrap());
        request
    }

    #[test]
    fn optionless_requests_can_still_dismiss() {
        let mut request = request();
        request.options.clear();

        let option = option_for(&request, IMPLICIT_DISMISS_OPTION_ID)
            .expect("implicit dismiss should be available");

        assert_eq!(option.id, IMPLICIT_DISMISS_OPTION_ID);
        assert_eq!(*option.kind, OptionKind::Dismiss);
    }

    #[test]
    fn blank_decision_text_is_signed_as_empty() {
        assert_eq!(trimmed_text("  \n\t  "), None);
    }

    #[test]
    fn signing_fails_when_request_digest_is_missing() {
        let mut request = request();
        request.request_digest = None;
        let key = StoredSigningKey::generate();
        let error = key
            .sign_decision(DecisionSigningRequest {
                request: &request,
                option_id: "approve",
                text: None,
                user_id: "user-1",
                device_id: "device-1",
            })
            .expect_err("missing digest should fail signing");

        assert!(error.to_string().contains("request digest is missing"));
    }

    #[test]
    fn signing_fails_when_option_is_not_available() {
        let request = request();
        let key = StoredSigningKey::generate();
        let error = key
            .sign_decision(DecisionSigningRequest {
                request: &request,
                option_id: "reject",
                text: None,
                user_id: "user-1",
                device_id: "device-1",
            })
            .expect_err("unknown option should fail signing");

        assert!(error.to_string().contains("option reject is not available"));
    }

    #[test]
    fn signing_fails_when_request_content_does_not_match_digest() {
        let mut request = request();
        request.title = "Tampered after the digest was computed".to_string();
        let key = StoredSigningKey::generate();
        let error = key
            .sign_decision(DecisionSigningRequest {
                request: &request,
                option_id: "approve",
                text: None,
                user_id: "user-1",
                device_id: "device-1",
            })
            .expect_err("a stale digest must fail signing");

        assert!(error
            .to_string()
            .contains("does not match request content"));
    }

    #[test]
    fn generated_public_key_uses_uncompressed_x963_bytes() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        let key = StoredSigningKey::generate();
        let public_key = key
            .device_signing_key()
            .expect("generated key should expose a public key");
        let bytes = URL_SAFE_NO_PAD
            .decode(public_key.public_key)
            .expect("public key should be base64url encoded");

        assert_eq!(bytes.len(), 65);
        assert_eq!(bytes[0], 4);
    }
}
