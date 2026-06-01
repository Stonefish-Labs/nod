use std::io::Cursor;

use appattest::attestation::Attestation;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use ciborium::value::Value;
use sha2::{Digest, Sha256};

use crate::{
    config::{AppAttestEnvironment, AppleAppAttestConfig},
    models::{
        DeviceAttestationRecord, DeviceAttestationRequest, EnrollDeviceRequest,
        FailedDeviceAttestation, VerifiedDeviceAttestation,
    },
    signing,
};

pub const APPLE_APP_ATTEST_PROVIDER: &str = "apple_app_attest";
const MAX_ATTESTATION_OBJECT_BYTES: usize = 64 * 1024;
const DEVELOPMENT_AAGUID: [u8; 16] = *b"appattestdevelop";
const DEVELOPMENT_SANDBOX_AAGUID: [u8; 16] = *b"appattestsandbox";
const PRODUCTION_AAGUID: [u8; 16] = *b"appattest\0\0\0\0\0\0\0";

const APPLE_APP_ATTEST_ROOT_CA_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIICITCCAaegAwIBAgIQC/O+DvHN0uD7jG5yH2IXmDAKBggqhkjOPQQDAzBSMSYw\n",
    "JAYDVQQDDB1BcHBsZSBBcHAgQXR0ZXN0YXRpb24gUm9vdCBDQTETMBEGA1UECgwK\n",
    "QXBwbGUgSW5jLjETMBEGA1UECAwKQ2FsaWZvcm5pYTAeFw0yMDAzMTgxODMyNTNa\n",
    "Fw00NTAzMTUwMDAwMDBaMFIxJjAkBgNVBAMMHUFwcGxlIEFwcCBBdHRlc3RhdGlv\n",
    "biBSb290IENBMRMwEQYDVQQKDApBcHBsZSBJbmMuMRMwEQYDVQQIDApDYWxpZm9y\n",
    "bmlhMHYwEAYHKoZIzj0CAQYFK4EEACIDYgAERTHhmLW07ATaFQIEVwTtT4dyctdh\n",
    "NbJhFs/Ii2FdCgAHGbpphY3+d8qjuDngIN3WVhQUBHAoMeQ/cLiP1sOUtgjqK9au\n",
    "Yen1mMEvRq9Sk3Jm5X8U62H+xTD3FE9TgS41o0IwQDAPBgNVHRMBAf8EBTADAQH/\n",
    "MB0GA1UdDgQWBBSskRBTM72+aEH/pwyp5frq5eWKoTAOBgNVHQ8BAf8EBAMCAQYw\n",
    "CgYIKoZIzj0EAwMDaAAwZQIwQgFGnByvsiVbpTKwSga0kP0e8EeDS4+sQmTvb7vn\n",
    "53O5+FRXgeLhpJ06ysC5PrOyAjEAp5U4xDgEgllF7En3VcE3iexZZtKeYnpqtijV\n",
    "oyFraWVIyd/dganmrduC1bmTBGwD\n",
    "-----END CERTIFICATE-----\n",
);

#[derive(Debug, Clone)]
struct ParsedAttestationObject {
    auth_data: Vec<u8>,
    receipt: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
struct ParsedAuthData {
    counter: u32,
    aaguid: [u8; 16],
}

struct EnrollmentAttestationVerifier<'a> {
    config: &'a AppleAppAttestConfig,
    enrollment: &'a EnrollDeviceRequest,
    attestation: &'a DeviceAttestationRequest,
    provider: &'a str,
    key_id: &'a str,
    team_id: Option<&'a str>,
}

struct VerificationMaterial<'a> {
    cbor: &'a [u8],
    parsed: &'a ParsedAttestationObject,
    auth_data: ParsedAuthData,
    client_data: &'a str,
    team_id: &'a str,
}

pub fn verify_enrollment_attestation(
    config: &AppleAppAttestConfig,
    req: &EnrollDeviceRequest,
) -> Option<DeviceAttestationRecord> {
    EnrollmentAttestationVerifier::new(config, req).map(|verifier| verifier.verify())
}

impl<'a> EnrollmentAttestationVerifier<'a> {
    fn new(config: &'a AppleAppAttestConfig, enrollment: &'a EnrollDeviceRequest) -> Option<Self> {
        let attestation = enrollment.attestation.as_ref()?;
        Some(Self {
            config,
            enrollment,
            attestation,
            provider: attestation.provider.trim(),
            key_id: attestation.key_id.trim(),
            team_id: config.team_id.as_deref().map(str::trim),
        })
    }

    fn verify(&self) -> DeviceAttestationRecord {
        if self.provider != APPLE_APP_ATTEST_PROVIDER {
            return self.failed("unsupported attestation provider");
        }
        if self.key_id.is_empty() {
            return self.failed_without_key("attestation key id is required");
        }
        if !self.config.configured() {
            return self.failed("Apple App Attest is not configured");
        }

        match self.verify_configured_attestation() {
            Ok(record) => record,
            Err(reason) => self.failed(reason),
        }
    }

    fn verify_configured_attestation(&self) -> Result<DeviceAttestationRecord, &'static str> {
        let cbor = decode_attestation_object(&self.attestation.attestation_object)?;
        let parsed = parse_attestation_object(&cbor)?;
        let auth_data = parse_auth_data(&parsed.auth_data)?;
        if !environment_matches(self.config.environment, &auth_data.aaguid) {
            return Err("App Attest environment mismatch");
        }

        let team_id = self.configured_team_id()?;
        let client_data = enrollment_client_data(self.enrollment, self.key_id);
        self.verify_bundle_ids(VerificationMaterial {
            cbor: &cbor,
            parsed: &parsed,
            auth_data,
            client_data: &client_data,
            team_id,
        })
    }

    fn verify_bundle_ids(
        &self,
        material: VerificationMaterial<'_>,
    ) -> Result<DeviceAttestationRecord, &'static str> {
        for bundle_id in self.config.normalized_bundle_ids() {
            let app_id = format!("{}.{}", material.team_id, bundle_id);
            let attestation = Attestation::from_cbor_bytes(material.cbor)
                .map_err(|_| "attestation object is invalid")?;
            match attestation.verify(
                material.client_data,
                &app_id,
                self.key_id,
                APPLE_APP_ATTEST_ROOT_CA_PEM.as_bytes(),
            ) {
                Ok((public_key, receipt)) => {
                    self.verify_native_app_id(&bundle_id)?;
                    return Ok(DeviceAttestationRecord::verified(
                        VerifiedDeviceAttestation {
                            provider: self.provider.to_string(),
                            key_id: self.key_id.to_string(),
                            team_id: material.team_id.to_string(),
                            bundle_id,
                            environment: self.config.environment.as_str().to_string(),
                            public_key: URL_SAFE_NO_PAD.encode(public_key),
                            counter: i64::from(material.auth_data.counter),
                            receipt_hash: receipt_hash(material.parsed, receipt),
                        },
                    ));
                }
                Err(err) => {
                    tracing::debug!(error = ?err, app_id, "Apple App Attest verification failed");
                }
            }
        }
        Err("App Attest verification failed")
    }

    fn verify_native_app_id(&self, verified_bundle_id: &str) -> Result<(), &'static str> {
        let Some(native_app_id) = normalized_optional(self.enrollment.native_app_id.as_deref())
        else {
            return Ok(());
        };
        if native_app_id == verified_bundle_id {
            Ok(())
        } else {
            Err("native app id does not match App Attest bundle id")
        }
    }

    fn configured_team_id(&self) -> Result<&'a str, &'static str> {
        self.team_id
            .filter(|team_id| !team_id.is_empty())
            .ok_or("Apple App Attest is not configured")
    }

    fn failed(&self, reason: &str) -> DeviceAttestationRecord {
        self.failed_with_key_id(non_empty(self.key_id), reason)
    }

    fn failed_without_key(&self, reason: &str) -> DeviceAttestationRecord {
        self.failed_with_key_id(None, reason)
    }

    fn failed_with_key_id(&self, key_id: Option<&str>, reason: &str) -> DeviceAttestationRecord {
        DeviceAttestationRecord::failed(FailedDeviceAttestation {
            provider: self.provider.to_string(),
            key_id: key_id.map(ToOwned::to_owned),
            team_id: self.team_id.map(ToOwned::to_owned),
            environment: Some(self.config.environment.as_str().to_string()),
            reason: reason.to_string(),
        })
    }
}

pub fn enrollment_client_data(req: &EnrollDeviceRequest, app_attest_key_id: &str) -> String {
    // The attested challenge binds the enrollment code, device identity, push route, and signing key.
    let signing_key = req.signing_key.as_ref();
    let signing_key_id = signing_key
        .map(|key| key.key_id.trim())
        .filter(|key_id| !key_id.is_empty())
        .unwrap_or_default();
    let signing_algorithm = signing_key
        .map(|key| key.algorithm.as_str())
        .filter(|algorithm| !algorithm.is_empty())
        .unwrap_or(signing::DEFAULT_ALGORITHM);
    let signing_public_key_hash = signing_key
        .map(|key| sha256_hex(key.public_key.trim().as_bytes()))
        .unwrap_or_default();
    format!(
        concat!(
            "nod-enrollment-v1\n",
            "code_sha256:{code_sha256}\n",
            "device_name_sha256:{device_name_sha256}\n",
            "platform:{platform}\n",
            "push_provider:{push_provider}\n",
            "push_token_sha256:{push_token_sha256}\n",
            "signing_key_id:{signing_key_id}\n",
            "signing_key_algorithm:{signing_algorithm}\n",
            "signing_public_key_sha256:{signing_public_key_hash}\n",
            "attestation_provider:{attestation_provider}\n",
            "attestation_key_id:{attestation_key_id}\n"
        ),
        code_sha256 = sha256_hex(req.code.trim().as_bytes()),
        device_name_sha256 = sha256_hex(req.device_name.trim().as_bytes()),
        platform = req.platform.as_str(),
        push_provider = normalized_optional(req.push_provider.as_deref()).unwrap_or_default(),
        push_token_sha256 = normalized_optional(req.push_token.as_deref())
            .map(|token| sha256_hex(token.as_bytes()))
            .unwrap_or_default(),
        signing_key_id = signing_key_id,
        signing_algorithm = signing_algorithm,
        signing_public_key_hash = signing_public_key_hash,
        attestation_provider = APPLE_APP_ATTEST_PROVIDER,
        attestation_key_id = app_attest_key_id.trim(),
    )
}

fn decode_attestation_object(value: &str) -> Result<Vec<u8>, &'static str> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_ATTESTATION_OBJECT_BYTES {
        return Err("attestation object is invalid");
    }
    URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| URL_SAFE.decode(value))
        .or_else(|_| STANDARD.decode(value))
        .map_err(|_| "attestation object is invalid")
}

fn parse_attestation_object(cbor: &[u8]) -> Result<ParsedAttestationObject, &'static str> {
    let value: Value = ciborium::from_reader(&mut Cursor::new(cbor))
        .map_err(|_| "attestation object is invalid")?;
    let fmt = text_field(&value, "fmt").ok_or("attestation object is invalid")?;
    if fmt != "apple-appattest" {
        return Err("attestation object has unsupported format");
    }
    let auth_data = bytes_field(&value, "authData")
        .ok_or("attestation object is missing authenticator data")?
        .to_vec();
    let receipt = map_field(&value, "attStmt").and_then(|att_stmt| {
        bytes_field(att_stmt, "receipt")
            .or_else(|| bytes_field(att_stmt, "receiptData"))
            .map(ToOwned::to_owned)
    });
    Ok(ParsedAttestationObject { auth_data, receipt })
}

fn parse_auth_data(auth_data: &[u8]) -> Result<ParsedAuthData, &'static str> {
    // WebAuthn authData is byte-packed, so keep offsets explicit for App Attest AAGUID validation.
    const RP_ID_HASH_LEN: usize = 32;
    const FLAGS_LEN: usize = 1;
    const COUNTER_LEN: usize = 4;
    const AAGUID_LEN: usize = 16;
    const CREDENTIAL_ID_LEN_BYTES: usize = 2;
    const ATTESTED_CREDENTIAL_DATA_FLAG: u8 = 0x40;

    let header_len = RP_ID_HASH_LEN + FLAGS_LEN + COUNTER_LEN;
    if auth_data.len() < header_len + AAGUID_LEN + CREDENTIAL_ID_LEN_BYTES {
        return Err("authenticator data is invalid");
    }
    let flags = auth_data[RP_ID_HASH_LEN];
    if flags & ATTESTED_CREDENTIAL_DATA_FLAG == 0 {
        return Err("authenticator data is missing attested credential data");
    }
    let counter_start = RP_ID_HASH_LEN + FLAGS_LEN;
    let counter = u32::from_be_bytes(
        auth_data[counter_start..counter_start + COUNTER_LEN]
            .try_into()
            .map_err(|_| "authenticator data is invalid")?,
    );
    if counter != 0 {
        return Err("authenticator counter is invalid");
    }

    let aaguid_start = header_len;
    let aaguid = auth_data[aaguid_start..aaguid_start + AAGUID_LEN]
        .try_into()
        .map_err(|_| "authenticator data is invalid")?;
    Ok(ParsedAuthData { counter, aaguid })
}

fn environment_matches(environment: AppAttestEnvironment, aaguid: &[u8; 16]) -> bool {
    match environment {
        AppAttestEnvironment::Development => {
            aaguid == &DEVELOPMENT_AAGUID || aaguid == &DEVELOPMENT_SANDBOX_AAGUID
        }
        AppAttestEnvironment::Production => aaguid == &PRODUCTION_AAGUID,
    }
}

fn map_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Map(entries) => entries.iter().find_map(|(entry_key, entry_value)| {
            matches!(entry_key, Value::Text(text) if text == key).then_some(entry_value)
        }),
        _ => None,
    }
}

fn text_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    match map_field(value, key) {
        Some(Value::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

fn bytes_field<'a>(value: &'a Value, key: &str) -> Option<&'a [u8]> {
    match map_field(value, key) {
        Some(Value::Bytes(bytes)) => Some(bytes.as_slice()),
        _ => None,
    }
}

fn normalized_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn receipt_hash(parsed: &ParsedAttestationObject, verified_receipt: &[u8]) -> Option<String> {
    if verified_receipt.is_empty() {
        return parsed.receipt.as_deref().map(sha256_hex);
    }
    Some(sha256_hex(verified_receipt))
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DevicePlatform, DeviceSigningKeyRequest, EnrollDeviceRequest};

    #[test]
    fn enrollment_client_data_has_stable_shape() {
        let req = EnrollDeviceRequest {
            code: " ABCD1234 ".to_string(),
            device_name: " iPhone ".to_string(),
            platform: DevicePlatform::Ios,
            native_app_id: Some(" com.batteryshark.Nod ".to_string()),
            push_provider: Some(" apple_apns ".to_string()),
            push_token: Some(" provider-token ".to_string()),
            signing_key: Some(DeviceSigningKeyRequest {
                key_id: "device-key-id".to_string(),
                algorithm: signing::DEFAULT_ALGORITHM.to_string(),
                public_key: "base64url-public-key".to_string(),
            }),
            attestation: None,
        };

        assert_eq!(
            enrollment_client_data(&req, "app-attest-key"),
            concat!(
                "nod-enrollment-v1\n",
                "code_sha256:1635c8525afbae58c37bede3c9440844e9143727cc7c160bed665ec378d8a262\n",
                "device_name_sha256:38fdf519314e3151d7e7f6ef456f327b78ddb84bc457bdb0d49bce0b1fc3c959\n",
                "platform:ios\n",
                "push_provider:apple_apns\n",
                "push_token_sha256:2ad21144ec11edbd553556e1dcd9a79383adbf4ae0e14266a19977edc3de9257\n",
                "signing_key_id:device-key-id\n",
                "signing_key_algorithm:p256_ecdsa_sha256\n",
                "signing_public_key_sha256:b848efd33d347196ccd5140f15975ed8d1f91cb2e99e2571fa5bd09282d5cc6f\n",
                "attestation_provider:apple_app_attest\n",
                "attestation_key_id:app-attest-key\n"
            )
        );
    }
}
