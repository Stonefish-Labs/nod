use std::{env, fmt, net::SocketAddr, path::PathBuf};

use anyhow::{bail, Context};

const DEFAULT_BIND: &str = "127.0.0.1:8768";
const BIND_ENV: &str = "NOD_APNS_RELAY_BIND";
const SERVER_CERT_ENV: &str = "NOD_APNS_RELAY_SERVER_CERT_PATH";
const SERVER_KEY_ENV: &str = "NOD_APNS_RELAY_SERVER_KEY_PATH";
const CLIENT_CA_CERT_ENV: &str = "NOD_APNS_RELAY_CLIENT_CA_CERT_PATH";
const TEAM_ID_ENV: &str = "NOD_APNS_RELAY_TEAM_ID";
const KEY_ID_ENV: &str = "NOD_APNS_RELAY_KEY_ID";
const BUNDLE_ID_ENV: &str = "NOD_APNS_RELAY_BUNDLE_ID";
const PRIVATE_KEY_ENV: &str = "NOD_APNS_RELAY_PRIVATE_KEY_PATH";
const ENVIRONMENT_ENV: &str = "NOD_APNS_RELAY_ENVIRONMENT";

/// Runtime configuration loaded from `NOD_APNS_RELAY_*` environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub tls: TlsConfig,
    pub apns: ApnsConfig,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        Self::from_source(&ProcessEnv)
    }

    fn from_source(source: &impl EnvSource) -> anyhow::Result<Self> {
        Ok(Self {
            bind: bind_addr(source)?,
            tls: TlsConfig::from_source(source)?,
            apns: ApnsConfig::from_source(source)?,
        })
    }
}

/// File paths for the relay server identity and trusted client CA bundle.
#[derive(Clone)]
pub struct TlsConfig {
    pub server_cert_path: PathBuf,
    pub server_key_path: PathBuf,
    pub client_ca_cert_path: PathBuf,
}

impl TlsConfig {
    fn from_source(source: &impl EnvSource) -> anyhow::Result<Self> {
        Ok(Self {
            server_cert_path: required_path(source, SERVER_CERT_ENV)?,
            server_key_path: required_path(source, SERVER_KEY_ENV)?,
            client_ca_cert_path: required_path(source, CLIENT_CA_CERT_ENV)?,
        })
    }
}

impl fmt::Debug for TlsConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TlsConfig")
            .field("server_cert_path", &self.server_cert_path)
            .field("server_key_path", &self.server_key_path)
            .field("client_ca_cert_path", &self.client_ca_cert_path)
            .finish()
    }
}

/// APNs identity and endpoint selection for outbound delivery.
#[derive(Debug, Clone)]
pub struct ApnsConfig {
    pub bundle_id: String,
    pub environment: ApnsEnvironment,
    pub credentials: ApnsCredentials,
}

impl ApnsConfig {
    fn from_source(source: &impl EnvSource) -> anyhow::Result<Self> {
        Ok(Self {
            bundle_id: required_text(source, BUNDLE_ID_ENV)?,
            environment: apns_environment(source)?,
            credentials: ApnsCredentials::from_source(source)?,
        })
    }
}

/// Apple APNs endpoint family used for outbound pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApnsEnvironment {
    Production,
    Sandbox,
}

impl ApnsEnvironment {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value.trim() {
            "production" => Ok(Self::Production),
            "sandbox" => Ok(Self::Sandbox),
            other => {
                bail!("{ENVIRONMENT_ENV} must be production or sandbox, got {other:?}")
            }
        }
    }

    pub fn endpoint(self) -> &'static str {
        match self {
            Self::Production => "https://api.push.apple.com",
            Self::Sandbox => "https://api.sandbox.push.apple.com",
        }
    }
}

/// Signing material used to create APNs provider tokens.
#[derive(Clone)]
pub struct ApnsCredentials {
    pub team_id: String,
    pub key_id: String,
    pub private_key_path: PathBuf,
}

impl ApnsCredentials {
    fn from_source(source: &impl EnvSource) -> anyhow::Result<Self> {
        Ok(Self {
            team_id: required_text(source, TEAM_ID_ENV)?,
            key_id: required_text(source, KEY_ID_ENV)?,
            private_key_path: required_path(source, PRIVATE_KEY_ENV)?,
        })
    }
}

impl fmt::Debug for ApnsCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redact Apple account identifiers so incidental Debug logging cannot expose them.
        formatter
            .debug_struct("ApnsCredentials")
            .field("team_id_configured", &true)
            .field("key_id_configured", &true)
            .field("private_key_path", &self.private_key_path)
            .finish()
    }
}

trait EnvSource {
    fn value(&self, name: &str) -> Option<String>;
}

struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn value(&self, name: &str) -> Option<String> {
        env::var(name).ok()
    }
}

fn bind_addr(source: &impl EnvSource) -> anyhow::Result<SocketAddr> {
    let value = optional_text(source, BIND_ENV).unwrap_or_else(|| DEFAULT_BIND.to_string());
    value
        .parse()
        .with_context(|| format!("{BIND_ENV} must be a socket address"))
}

fn apns_environment(source: &impl EnvSource) -> anyhow::Result<ApnsEnvironment> {
    optional_text(source, ENVIRONMENT_ENV).map_or(Ok(ApnsEnvironment::Production), |value| {
        ApnsEnvironment::parse(&value)
    })
}

fn required_text(source: &impl EnvSource, name: &str) -> anyhow::Result<String> {
    optional_text(source, name).ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn required_path(source: &impl EnvSource, name: &str) -> anyhow::Result<PathBuf> {
    required_text(source, name).map(PathBuf::from)
}

fn optional_text(source: &impl EnvSource, name: &str) -> Option<String> {
    source
        .value(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn config_loads_required_values_with_default_bind_address() {
        let config = Config::from_source(&complete_env()).unwrap();

        assert_eq!(config.bind, "127.0.0.1:8768".parse().unwrap());
        assert_eq!(config.apns.bundle_id, "com.example.NodTests");
        assert_eq!(config.apns.environment, ApnsEnvironment::Production);
        assert_eq!(
            config.tls.client_ca_cert_path,
            PathBuf::from("tests/fixtures/mtls/client-ca.crt")
        );
    }

    #[test]
    fn config_parses_custom_bind_address() {
        let config = Config::from_source(&complete_env().with(BIND_ENV, "127.0.0.1:9000")).unwrap();

        assert_eq!(config.bind, "127.0.0.1:9000".parse().unwrap());
    }

    #[test]
    fn config_rejects_invalid_bind_address() {
        let err = Config::from_source(&complete_env().with(BIND_ENV, "not-a-socket"))
            .unwrap_err()
            .to_string();

        assert!(err.contains(BIND_ENV));
    }

    #[test]
    fn config_requires_mtls_files() {
        let err = Config::from_source(&complete_env().without(SERVER_KEY_ENV))
            .unwrap_err()
            .to_string();

        assert!(err.contains(SERVER_KEY_ENV));
    }

    #[test]
    fn config_requires_apns_credentials() {
        let err = Config::from_source(&complete_env().without(TEAM_ID_ENV))
            .unwrap_err()
            .to_string();

        assert!(err.contains(TEAM_ID_ENV));
    }

    #[test]
    fn apns_environment_accepts_known_values() {
        assert_eq!(
            ApnsEnvironment::parse("production").unwrap(),
            ApnsEnvironment::Production
        );
        assert_eq!(
            ApnsEnvironment::parse(" sandbox ").unwrap(),
            ApnsEnvironment::Sandbox
        );
    }

    #[test]
    fn apns_environment_rejects_unknown_values() {
        let err = ApnsEnvironment::parse("staging").unwrap_err().to_string();

        assert!(err.contains(ENVIRONMENT_ENV));
    }

    struct TestEnv {
        values: BTreeMap<&'static str, &'static str>,
    }

    impl TestEnv {
        fn with(mut self, name: &'static str, value: &'static str) -> Self {
            self.values.insert(name, value);
            self
        }

        fn without(mut self, name: &'static str) -> Self {
            self.values.remove(name);
            self
        }
    }

    impl EnvSource for TestEnv {
        fn value(&self, name: &str) -> Option<String> {
            self.values.get(name).map(|value| (*value).to_string())
        }
    }

    fn complete_env() -> TestEnv {
        TestEnv {
            values: BTreeMap::from([
                (SERVER_CERT_ENV, "tests/fixtures/mtls/server.crt"),
                (SERVER_KEY_ENV, "tests/fixtures/mtls/server.key"),
                (CLIENT_CA_CERT_ENV, "tests/fixtures/mtls/client-ca.crt"),
                (TEAM_ID_ENV, "TEAMID"),
                (KEY_ID_ENV, "KEYID"),
                (BUNDLE_ID_ENV, "com.example.NodTests"),
                (PRIVATE_KEY_ENV, "tests/fixtures/mtls/apns-auth-key.p8"),
            ]),
        }
    }
}
