use std::sync::Arc;

use async_trait::async_trait;
use axum::serve::Listener;
use nod_apns_relay::{
    config::TlsConfig,
    relay::{ApnsDelivery, DynApnsDelivery, RelayNotification, RelayPolicy},
    router,
    tls::MtlsListener,
};
use reqwest::{Certificate, Client, Identity};
use tokio::sync::{oneshot, Mutex};

#[derive(Clone, Default)]
struct RecordingProvider {
    requests: Arc<Mutex<Vec<RelayNotification>>>,
}

#[async_trait]
impl ApnsDelivery for RecordingProvider {
    async fn send(&self, notification: &RelayNotification) -> anyhow::Result<()> {
        self.requests.lock().await.push(notification.clone());
        Ok(())
    }
}

#[tokio::test]
async fn accepts_trusted_client_certificate() {
    let provider = RecordingProvider::default();
    let (url, shutdown) = spawn_proxy(Arc::new(provider.clone())).await;
    let response = trusted_client()
        .post(format!("{url}/v1/notifications"))
        .json(&valid_request())
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(provider.requests.lock().await.len(), 1);
    let _ = shutdown.send(());
}

#[tokio::test]
async fn rejects_missing_client_certificate() {
    let provider = RecordingProvider::default();
    let (url, shutdown) = spawn_proxy(Arc::new(provider)).await;
    let err = client_without_identity()
        .get(format!("{url}/health"))
        .send()
        .await
        .unwrap_err();

    assert!(err.is_request());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn rejects_untrusted_client_certificate() {
    let provider = RecordingProvider::default();
    let (url, shutdown) = spawn_proxy(Arc::new(provider)).await;
    let err = untrusted_client()
        .get(format!("{url}/health"))
        .send()
        .await
        .unwrap_err();

    assert!(err.is_request());
    let _ = shutdown.send(());
}

async fn spawn_proxy(provider: DynApnsDelivery) -> (String, oneshot::Sender<()>) {
    let config = tls_config();
    let addr = "127.0.0.1:0".parse().unwrap();
    let listener = MtlsListener::bind(addr, &config).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(provider, RelayPolicy::new("com.example.NodTests"));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (format!("https://localhost:{}", addr.port()), shutdown_tx)
}

fn trusted_client() -> Client {
    mtls_client(
        "tests/fixtures/mtls/client.crt",
        "tests/fixtures/mtls/client.key",
    )
}

fn untrusted_client() -> Client {
    mtls_client(
        "tests/fixtures/mtls/untrusted-client.crt",
        "tests/fixtures/mtls/untrusted-client.key",
    )
}

fn client_without_identity() -> Client {
    Client::builder()
        .add_root_certificate(server_ca())
        .build()
        .unwrap()
}

fn mtls_client(cert_path: &str, key_path: &str) -> Client {
    let mut identity_pem = std::fs::read(cert_path).unwrap();
    // reqwest expects the client certificate and private key in one PEM blob.
    identity_pem.extend(std::fs::read(key_path).unwrap());
    Client::builder()
        .add_root_certificate(server_ca())
        .identity(Identity::from_pem(&identity_pem).unwrap())
        .build()
        .unwrap()
}

fn server_ca() -> Certificate {
    Certificate::from_pem(&std::fs::read("tests/fixtures/mtls/server-ca.crt").unwrap()).unwrap()
}

fn tls_config() -> TlsConfig {
    TlsConfig {
        server_cert_path: "tests/fixtures/mtls/server.crt".into(),
        server_key_path: "tests/fixtures/mtls/server.key".into(),
        client_ca_cert_path: "tests/fixtures/mtls/client-ca.crt".into(),
    }
}

fn valid_request() -> serde_json::Value {
    serde_json::json!({
        "target": {
            "platform": "ios",
            "native_app_id": "com.example.NodTests",
            "token": "device-token"
        },
        "notification": {
            "title": "Deploy",
            "body": "Production deploy is waiting",
            "sound": "default",
            "thread_id": "default",
            "category": "NOD_APPROVAL"
        },
        "metadata": {
            "request_id": "request-1",
            "source_id": "default"
        }
    })
}
