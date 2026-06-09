use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::models::ServerProfile;
use crate::signing::StoredSigningKey;

const KEYRING_SERVICE: &str = "nod-client-core";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConfig {
    #[serde(default)]
    pub servers: Vec<ServerProfile>,
    #[serde(default)]
    pub selected_server_id: Option<String>,
    #[serde(default = "default_notification_sound")]
    pub notification_sound: String,
    #[serde(default)]
    pub insecure_tokens: BTreeMap<String, String>,
    #[serde(default)]
    pub insecure_signing_keys: BTreeMap<String, StoredSigningKey>,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            selected_server_id: None,
            notification_sound: default_notification_sound(),
            insecure_tokens: BTreeMap::new(),
            insecure_signing_keys: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
    credentials: CredentialStore,
}

impl Store {
    pub fn new() -> Result<Self> {
        let state_dir = if let Ok(path) = env::var("NOD_CLIENT_CORE_STATE_DIR") {
            PathBuf::from(path)
        } else {
            ProjectDirs::from("com", "Stonefish Labs", "Nod")
                .context("could not resolve user config directory")?
                .config_dir()
                .to_path_buf()
        };
        Ok(Self {
            path: state_dir.join("client-core.json"),
            credentials: CredentialStore::from_env(),
        })
    }

    pub async fn load(&self) -> Result<PersistedConfig> {
        if !self.path.exists() {
            return Ok(PersistedConfig::default());
        }
        let raw = fs::read(&self.path)
            .await
            .with_context(|| format!("read {}", self.path.display()))?;
        let config: PersistedConfig = serde_json::from_slice(&raw)?;
        Ok(config)
    }

    pub async fn save(&self, mut config: PersistedConfig) -> Result<()> {
        self.credentials.remove_external_credentials(&mut config);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let raw = serde_json::to_vec_pretty(&config)?;
        fs::write(&self.path, raw)
            .await
            .with_context(|| format!("write {}", self.path.display()))?;
        Ok(())
    }

    pub async fn save_token(
        &self,
        config: &mut PersistedConfig,
        server_id: &str,
        token: &str,
    ) -> Result<()> {
        self.credentials.save_token(config, server_id, token)
    }

    pub fn load_token(&self, config: &PersistedConfig, server_id: &str) -> Option<String> {
        self.credentials.load_token(config, server_id)
    }

    pub async fn delete_token(&self, config: &mut PersistedConfig, server_id: &str) -> Result<()> {
        self.credentials.delete_token(config, server_id)
    }

    pub async fn save_signing_key(
        &self,
        config: &mut PersistedConfig,
        server_id: &str,
        signing_key: &StoredSigningKey,
    ) -> Result<()> {
        self.credentials
            .save_signing_key(config, server_id, signing_key)
    }

    pub fn load_signing_key(
        &self,
        config: &PersistedConfig,
        server_id: &str,
    ) -> Option<StoredSigningKey> {
        self.credentials.load_signing_key(config, server_id)
    }

    pub async fn delete_signing_key(
        &self,
        config: &mut PersistedConfig,
        server_id: &str,
    ) -> Result<()> {
        self.credentials.delete_signing_key(config, server_id)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

}

#[derive(Debug, Clone)]
struct CredentialStore {
    use_config_file: bool,
}

impl CredentialStore {
    fn from_env() -> Self {
        Self {
            use_config_file: env::var("NOD_CLIENT_CORE_INSECURE_TOKEN_STORE").is_ok(),
        }
    }

    fn remove_external_credentials(&self, config: &mut PersistedConfig) {
        if self.use_config_file {
            return;
        }
        config.insecure_tokens.clear();
        config.insecure_signing_keys.clear();
    }

    fn save_token(&self, config: &mut PersistedConfig, server_id: &str, token: &str) -> Result<()> {
        if self.use_config_file {
            config
                .insecure_tokens
                .insert(server_id.to_string(), token.to_string());
            return Ok(());
        }
        set_keyring_password(&token_account(server_id), token)
    }

    fn load_token(&self, config: &PersistedConfig, server_id: &str) -> Option<String> {
        if self.use_config_file {
            return config.insecure_tokens.get(server_id).cloned();
        }
        keyring_password(&token_account(server_id))
    }

    fn delete_token(&self, config: &mut PersistedConfig, server_id: &str) -> Result<()> {
        config.insecure_tokens.remove(server_id);
        if !self.use_config_file {
            delete_keyring_credential(&token_account(server_id));
        }
        Ok(())
    }

    fn save_signing_key(
        &self,
        config: &mut PersistedConfig,
        server_id: &str,
        signing_key: &StoredSigningKey,
    ) -> Result<()> {
        if self.use_config_file {
            config
                .insecure_signing_keys
                .insert(server_id.to_string(), signing_key.clone());
            return Ok(());
        }
        set_keyring_password(
            &signing_key_account(server_id),
            &serde_json::to_string(signing_key)?,
        )
    }

    fn load_signing_key(
        &self,
        config: &PersistedConfig,
        server_id: &str,
    ) -> Option<StoredSigningKey> {
        if self.use_config_file {
            return config.insecure_signing_keys.get(server_id).cloned();
        }
        keyring_password(&signing_key_account(server_id))
            .and_then(|raw| serde_json::from_str(&raw).ok())
    }

    fn delete_signing_key(&self, config: &mut PersistedConfig, server_id: &str) -> Result<()> {
        config.insecure_signing_keys.remove(server_id);
        if !self.use_config_file {
            delete_keyring_credential(&signing_key_account(server_id));
        }
        Ok(())
    }
}

fn set_keyring_password(account: &str, password: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)?;
    entry.set_password(password)?;
    Ok(())
}

fn keyring_password(account: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account).ok()?;
    entry.get_password().ok()
}

fn delete_keyring_credential(account: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, account) {
        let _ = entry.delete_credential();
    }
}

fn token_account(server_id: &str) -> String {
    format!("serverToken.{server_id}")
}

fn signing_key_account(server_id: &str) -> String {
    format!("decisionSigningKey.{server_id}")
}

fn default_notification_sound() -> String {
    "default".to_string()
}

