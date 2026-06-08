use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use ring::signature;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    error::ApiError,
    models::{DecisionRequest, Device, RequestOption, SubmitDecisionSignature},
};

pub const DEFAULT_ALGORITHM: &str = "p256_ecdsa_sha256";

pub fn verify_decision_signature(
    request: &DecisionRequest,
    option: &RequestOption,
    actor: &Device,
    text: Option<&str>,
    provided: &SubmitDecisionSignature,
) -> Result<(String, String), ApiError> {
    let Some(device_key_id) = actor.signing_key_id.as_deref() else {
        return Err(ApiError::BadRequest(
            "device does not have a registered signing key".to_string(),
        ));
    };
    let Some(public_key) = actor.signing_public_key.as_deref() else {
        return Err(ApiError::BadRequest(
            "device does not have a registered signing key".to_string(),
        ));
    };
    let algorithm = actor
        .signing_key_algorithm
        .as_deref()
        .unwrap_or(DEFAULT_ALGORITHM);
    if algorithm != DEFAULT_ALGORITHM || provided.algorithm != DEFAULT_ALGORITHM {
        return Err(ApiError::BadRequest(
            "unsupported decision signature algorithm".to_string(),
        ));
    }
    if provided.key_id != device_key_id {
        return Err(ApiError::BadRequest(
            "decision signature key_id does not match this device".to_string(),
        ));
    }
    if provided.nonce.trim().is_empty() || provided.nonce.len() > 160 {
        return Err(ApiError::BadRequest(
            "decision signature nonce is required".to_string(),
        ));
    }

    let request_digest = request_digest(request)?;
    if provided.request_digest != request_digest {
        return Err(ApiError::BadRequest(
            "decision signature request_digest does not match the request snapshot".to_string(),
        ));
    }
    let signed_at = DateTime::parse_from_rfc3339(&provided.signed_at)
        .map_err(|_| ApiError::BadRequest("decision signature signed_at is invalid".to_string()))?
        .with_timezone(&Utc);
    let signing_payload = decision_signing_payload(DecisionSigningPayload {
        request,
        option,
        actor,
        text,
        key_id: &provided.key_id,
        nonce: &provided.nonce,
        signed_at,
        request_digest: &request_digest,
    })?;
    let public_key = URL_SAFE_NO_PAD.decode(public_key).map_err(|_| {
        ApiError::BadRequest("registered signing public key is invalid".to_string())
    })?;
    let signature = URL_SAFE_NO_PAD
        .decode(&provided.signature)
        .map_err(|_| ApiError::BadRequest("decision signature is invalid".to_string()))?;

    let verifier =
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, public_key);
    verifier
        .verify(signing_payload.as_bytes(), &signature)
        .map_err(|_| ApiError::Forbidden)?;
    Ok((request_digest, signing_payload))
}

pub fn request_digest(request: &DecisionRequest) -> Result<String, ApiError> {
    // The digest covers the immutable prompt snapshot, not delivery state or later decisions.
    let options = request
        .options
        .iter()
        .map(|option| {
            json!({
                "id": option.id,
                "label": option.label,
                "kind": option.kind.as_str(),
                "style": option.style,
                "requires_text": option.requires_text,
                "text_placeholder": option.text_placeholder,
                "destructive": option.destructive,
                "foreground": option.foreground,
            })
        })
        .collect::<Vec<_>>();
    let fields = serde_json::to_string(&request.fields)?;
    let links = serde_json::to_string(&request.links)?;
    let options = serde_json::to_string(&options)?;
    let snapshot = format!(
        concat!(
            "nod-request-v1\n",
            "request_id:{request_id}\n",
            "source_id:{source_id}\n",
            "recipients:{recipients}\n",
            "decision_resolution:{decision_resolution}\n",
            "title_sha256:{title}\n",
            "summary_sha256:{summary}\n",
            "body_markdown_sha256:{body}\n",
            "fields_sha256:{fields}\n",
            "links_sha256:{links}\n",
            "image_url:{image_url}\n",
            "dedupe_key:{dedupe_key}\n",
            "expires_at:{expires_at}\n",
            "created_at:{created_at}\n",
            "options_sha256:{options}\n"
        ),
        request_id = request.id,
        source_id = request.source_id,
        recipients = request.recipients.join(","),
        decision_resolution = request.decision_resolution.as_str(),
        title = sha256_hex(request.title.as_bytes()),
        summary = sha256_hex(request.summary.as_bytes()),
        body = sha256_hex(request.body_markdown.as_bytes()),
        fields = sha256_hex(fields.as_bytes()),
        links = sha256_hex(links.as_bytes()),
        image_url = request.image_url.as_deref().unwrap_or(""),
        dedupe_key = request.dedupe_key.as_deref().unwrap_or(""),
        expires_at = request
            .expires_at
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_default(),
        created_at = request
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        options = sha256_hex(options.as_bytes()),
    );
    Ok(sha256_hex(snapshot.as_bytes()))
}

struct DecisionSigningPayload<'a> {
    request: &'a DecisionRequest,
    option: &'a RequestOption,
    actor: &'a Device,
    text: Option<&'a str>,
    key_id: &'a str,
    nonce: &'a str,
    signed_at: DateTime<Utc>,
    request_digest: &'a str,
}

fn decision_signing_payload(payload: DecisionSigningPayload<'_>) -> Result<String, ApiError> {
    // This string is signed by clients, so every field and newline is intentionally stable.
    Ok(format!(
        concat!(
            "nod-decision-v1\n",
            "request_id:{request_id}\n",
            "request_digest:{request_digest}\n",
            "option_id:{option_id}\n",
            "option_kind:{option_kind}\n",
            "user_id:{user_id}\n",
            "device_id:{device_id}\n",
            "key_id:{key_id}\n",
            "nonce:{nonce}\n",
            "signed_at:{signed_at}\n",
            "text_sha256:{text_sha256}\n"
        ),
        request_id = payload.request.id,
        request_digest = payload.request_digest,
        option_id = payload.option.id,
        option_kind = payload.option.kind.as_str(),
        user_id = payload.actor.user_id,
        device_id = payload.actor.id,
        key_id = payload.key_id,
        nonce = payload.nonce,
        signed_at = payload
            .signed_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        text_sha256 = sha256_hex(payload.text.unwrap_or("").as_bytes()),
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
