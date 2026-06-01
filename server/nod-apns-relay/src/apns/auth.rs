use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;

use crate::config::ApnsCredentials;

pub(crate) struct ApnsTokenSigner {
    team_id: String,
    key_id: String,
    encoding_key: EncodingKey,
}

impl ApnsTokenSigner {
    pub(crate) fn from_credentials(credentials: ApnsCredentials) -> anyhow::Result<Self> {
        let key = fs::read(credentials.private_key_path)?;
        Ok(Self {
            team_id: credentials.team_id,
            key_id: credentials.key_id,
            encoding_key: EncodingKey::from_ec_pem(&key)?,
        })
    }

    pub(crate) fn jwt(&self) -> anyhow::Result<String> {
        #[derive(Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            iat: u64,
        }

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        // Build a fresh provider token so `iat` stays inside Apple's validity
        // window across long relay uptime.
        let issued_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(jsonwebtoken::encode(
            &header,
            &Claims {
                iss: &self.team_id,
                iat: issued_at,
            },
            &self.encoding_key,
        )?)
    }
}
