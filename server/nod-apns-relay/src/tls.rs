use std::{io, net::SocketAddr, sync::Arc};

use axum::serve::Listener;
use rustls::{server::WebPkiClientVerifier, RootCertStore, ServerConfig as RustlsServerConfig};
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};

use crate::config::TlsConfig;

/// Axum listener that completes mTLS before handing connections to the router.
pub struct MtlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl MtlsListener {
    pub async fn bind(addr: SocketAddr, config: &TlsConfig) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let server_config = rustls_server_config(config)?;
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(Arc::new(server_config)),
        })
    }
}

impl Listener for MtlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(err) => {
                    tracing::warn!(error = %err, "failed to accept TCP connection");
                    continue;
                }
            };
            match self.acceptor.accept(stream).await {
                Ok(stream) => return (stream, addr),
                Err(err) => {
                    // A rejected client certificate fails before Axum sees a request,
                    // so the listener logs the handshake failure and waits for the next connection.
                    tracing::warn!(%addr, error = %err, "mTLS handshake failed");
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

/// Builds a rustls server config that requires clients to chain to the configured CA.
pub fn rustls_server_config(config: &TlsConfig) -> anyhow::Result<RustlsServerConfig> {
    install_default_crypto_provider();

    let certs = load_certificates(&config.server_cert_path)?;
    let key = PrivateKeyDer::from_pem_file(&config.server_key_path)?;
    let client_roots = Arc::new(load_root_store(&config.client_ca_cert_path)?);
    let client_verifier = WebPkiClientVerifier::builder(client_roots).build()?;
    Ok(RustlsServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)?)
}

fn load_root_store(path: impl AsRef<std::path::Path>) -> anyhow::Result<RootCertStore> {
    let certs = load_certificates(path)?;
    let mut roots = RootCertStore::empty();
    let (added, ignored) = roots.add_parsable_certificates(certs);
    if added == 0 {
        anyhow::bail!("CA certificate bundle did not contain usable certificates");
    }
    if ignored > 0 {
        tracing::warn!(ignored, "ignored malformed CA certificates");
    }
    Ok(roots)
}

fn load_certificates(
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let certs = CertificateDer::pem_file_iter(path)?.collect::<Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        anyhow::bail!("certificate file did not contain certificates");
    }
    Ok(certs)
}

fn install_default_crypto_provider() {
    // rustls installs crypto providers process-wide; repeated setup happens in
    // tests and returns an error after the first successful install.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustls_config_loads_mtls_files() {
        let config = TlsConfig {
            server_cert_path: "tests/fixtures/mtls/server.crt".into(),
            server_key_path: "tests/fixtures/mtls/server.key".into(),
            client_ca_cert_path: "tests/fixtures/mtls/client-ca.crt".into(),
        };

        let server_config = rustls_server_config(&config).unwrap();

        assert!(server_config.alpn_protocols.is_empty());
    }
}
