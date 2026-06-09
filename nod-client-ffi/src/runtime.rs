//! The async `NodClientRuntime` exposed to Swift.
//!
//! `nod-client-core` already owns the full client: an async JSON-RPC machine
//! (`handle_rpc(RpcRequest) -> RpcResponse`) plus an mpsc outbox of
//! `NodClientMessage` events. The TUI and the Tauri desktop drive it directly;
//! this is the same seam for Swift.
//!
//! The transport is deliberately JSON strings — byte-for-byte the same
//! request/response/event shapes the desktop already speaks over Tauri IPC — so
//! there is ONE client protocol and the Apple app decodes the very `ClientState`
//! / `Request` types the desktop decodes (already `#[typeshare]`'d). The Swift
//! side gets:
//!   - `NodClientObserver` — a foreign callback the runtime pushes events to;
//!   - `NodClient.request(json)` — async RPC in;
//!   - `NodClient.start()` — emit the initial ready + state.
//!
//! `#[uniffi::export(async_runtime = "tokio")]` wraps each future in
//! async-compat, giving nod-client-core's tokio I/O (reqwest, the sync
//! websocket) an ambient runtime and keeping `tokio::spawn`ed background tasks
//! (the outbox pump, the sync task) alive on async-compat's global runtime.
//!
//! NOTE (security, deliberately scoped): the runtime's decision signing still
//! uses nod-client-core's software `StoredSigningKey`. The Apple apps must NOT
//! regress off the Secure Enclave, so this transport is exposed and verified
//! first; the next step is a `DeviceSigner` port so Swift signs in the Secure
//! Enclave via a second foreign callback. Until that lands, NodKit keeps its own
//! enroll/submit path — nothing here ships software signing to Apple.

use std::sync::Arc;

use nod_client_core::{
    ForeignSigner, ForeignSignerKey, NodClientMessage, NodClientRuntime, RpcRequest, SignerBackend,
};
use serde_json::json;
use tokio::sync::{mpsc, Mutex};

/// Outbox depth — matches the desktop's runtime message buffer. Bounded so a
/// stalled observer applies backpressure rather than growing unbounded.
const OUTBOX_BUFFER: usize = 256;

/// Errors surfaced to Swift when the runtime cannot be constructed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ClientFfiError {
    #[error("failed to start the nod client runtime: {message}")]
    Runtime { message: String },
}

/// Foreign callback the runtime pushes serialized `NodClientMessage`s to. Each
/// `message` is a JSON object `{ "kind": ..., "payload": ... }` — the same
/// envelope the desktop receives over Tauri IPC. Swift decodes it and drives the
/// UI / native side effects (notifications, etc.).
#[uniffi::export(callback_interface)]
pub trait NodClientObserver: Send + Sync {
    fn on_message(&self, message: String);
}

/// Errors a Swift `NodDeviceSigner` can raise back across the FFI.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SignerCallbackError {
    #[error("device signer failed: {message}")]
    Failed { message: String },
}

/// The public identity of a Secure Enclave key, passed across the FFI. The
/// private key never crosses — only the id the server registers and the public
/// key it verifies against.
#[derive(uniffi::Record)]
pub struct NodDeviceKey {
    pub key_id: String,
    /// base64url uncompressed (x9.63) P-256 public key.
    pub public_key: String,
}

/// The Secure Enclave signing callback Swift implements. The runtime calls this
/// instead of generating/persisting a software key, so the Apple apps keep
/// non-exportable hardware keys while using all of nod-client-core. Keyed by
/// server profile id (each enrolled server has its own SE key). This is the
/// security-critical capability port: signing happens in hardware, the canonical
/// payload bytes are built in Rust.
#[uniffi::export(callback_interface)]
pub trait NodDeviceSigner: Send + Sync {
    /// Create (or fetch) the SE key for a freshly enrolling profile; return its
    /// public identity to register with the server.
    fn provision(&self, profile_id: String) -> Result<NodDeviceKey, SignerCallbackError>;
    /// The existing SE key for a profile, or `None` if there is none.
    fn signing_key(&self, profile_id: String)
        -> Result<Option<NodDeviceKey>, SignerCallbackError>;
    /// Sign the canonical decision payload bytes with the profile's SE key;
    /// return a base64url DER ECDSA signature.
    fn sign(&self, profile_id: String, payload: Vec<u8>)
        -> Result<String, SignerCallbackError>;
    /// Drop the SE key when a server is forgotten.
    fn remove(&self, profile_id: String) -> Result<(), SignerCallbackError>;
}

/// Adapts the Swift `NodDeviceSigner` callback to nod-client-core's
/// `ForeignSigner` port, mapping FFI errors to `anyhow`.
struct ForeignSignerBridge {
    callback: Box<dyn NodDeviceSigner>,
}

impl ForeignSigner for ForeignSignerBridge {
    fn provision(&self, profile_id: &str) -> anyhow::Result<ForeignSignerKey> {
        self.callback
            .provision(profile_id.to_string())
            .map(into_core_key)
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    fn signing_key(&self, profile_id: &str) -> anyhow::Result<Option<ForeignSignerKey>> {
        self.callback
            .signing_key(profile_id.to_string())
            .map(|key| key.map(into_core_key))
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    fn sign(&self, profile_id: &str, payload: &[u8]) -> anyhow::Result<String> {
        self.callback
            .sign(profile_id.to_string(), payload.to_vec())
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    fn remove(&self, profile_id: &str) -> anyhow::Result<()> {
        self.callback
            .remove(profile_id.to_string())
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }
}

fn into_core_key(key: NodDeviceKey) -> ForeignSignerKey {
    ForeignSignerKey {
        key_id: key.key_id,
        public_key: key.public_key,
    }
}

/// The Swift-facing handle to the shared client runtime.
#[derive(uniffi::Object)]
pub struct NodClient {
    inner: Arc<Mutex<NodClientRuntime>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl NodClient {
    /// Build the runtime and start pumping its outbox to `observer`. The pump is
    /// a detached task on async-compat's global tokio runtime, so events keep
    /// flowing between `request` calls (sync pushes, notification candidates).
    #[uniffi::constructor]
    pub async fn new(
        observer: Box<dyn NodClientObserver>,
        signer: Box<dyn NodDeviceSigner>,
    ) -> Result<Arc<Self>, ClientFfiError> {
        let (tx, mut rx) = mpsc::channel::<NodClientMessage>(OUTBOX_BUFFER);
        // Apple always signs in the Secure Enclave — the FFI mandates a signer,
        // so there is no software-key path to regress onto.
        let backend = SignerBackend::Foreign(Arc::new(ForeignSignerBridge { callback: signer }));
        let runtime = NodClientRuntime::with_signer_backend(tx, backend)
            .await
            .map_err(|error| ClientFfiError::Runtime {
                message: error.to_string(),
            })?;

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                // Serialization of NodClientMessage cannot fail in practice
                // (plain serde structs); fall back to a transient-error envelope
                // rather than dropping the event silently.
                let json = serde_json::to_string(&message).unwrap_or_else(|error| {
                    json!({
                        "kind": "transient_error",
                        "payload": { "message": format!("failed to encode runtime message: {error}") }
                    })
                    .to_string()
                });
                observer.on_message(json);
            }
        });

        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(runtime)),
        }))
    }

    /// Emit the initial `Ready` + `State` so a freshly attached observer renders
    /// immediately, before any RPC.
    pub async fn start(&self) {
        let runtime = self.inner.lock().await;
        runtime.emit_ready().await;
        runtime.emit_state().await;
    }

    /// Dispatch one JSON-RPC request and return the JSON-RPC response. `request`
    /// is `{ "id": ..., "method": ..., "params": ... }`; the reply is
    /// `{ "id": ..., "ok": ..., "result"|"error": ... }`. Malformed JSON yields a
    /// well-formed failure envelope rather than throwing across the FFI.
    pub async fn request(&self, request_json: String) -> String {
        let parsed: RpcRequest = match serde_json::from_str(&request_json) {
            Ok(request) => request,
            Err(error) => {
                return json!({
                    "id": serde_json::Value::Null,
                    "ok": false,
                    "error": format!("invalid rpc request json: {error}")
                })
                .to_string();
            }
        };
        let mut runtime = self.inner.lock().await;
        let response = runtime.handle_rpc(parsed).await;
        serde_json::to_string(&response).unwrap_or_else(|error| {
            json!({
                "id": serde_json::Value::Null,
                "ok": false,
                "error": format!("failed to encode rpc response: {error}")
            })
            .to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use super::*;

    /// Captures the observer callbacks so the test can assert the runtime pushed
    /// the expected envelopes.
    #[derive(Default)]
    struct RecordingObserver {
        messages: Arc<Mutex<Vec<String>>>,
        count: Arc<AtomicUsize>,
    }

    impl NodClientObserver for RecordingObserver {
        fn on_message(&self, message: String) {
            self.count.fetch_add(1, Ordering::SeqCst);
            self.messages.lock().unwrap().push(message);
        }
    }

    /// A signer the `state` RPC never invokes; present only to satisfy the
    /// constructor (the SE signing path itself is covered by nod-client-core's
    /// `foreign_signer_path_produces_verifiable_signature`).
    struct UnusedSigner;

    impl NodDeviceSigner for UnusedSigner {
        fn provision(&self, _profile_id: String) -> Result<NodDeviceKey, SignerCallbackError> {
            Err(SignerCallbackError::Failed {
                message: "unused".into(),
            })
        }
        fn signing_key(
            &self,
            _profile_id: String,
        ) -> Result<Option<NodDeviceKey>, SignerCallbackError> {
            Ok(None)
        }
        fn sign(
            &self,
            _profile_id: String,
            _payload: Vec<u8>,
        ) -> Result<String, SignerCallbackError> {
            Err(SignerCallbackError::Failed {
                message: "unused".into(),
            })
        }
        fn remove(&self, _profile_id: String) -> Result<(), SignerCallbackError> {
            Ok(())
        }
    }

    /// Drive the full FFI surface against a hermetic, file-backed store (no
    /// keychain, temp state dir): construct → start → a `state` RPC, and assert
    /// the observer saw a `ready` then a `state` event and the RPC round-trips.
    #[tokio::test]
    async fn ffi_runtime_starts_and_round_trips_state_rpc() {
        let temp = std::env::temp_dir().join(format!("nod-ffi-test-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        std::env::set_var("NOD_CLIENT_CORE_STATE_DIR", &temp);
        std::env::set_var("NOD_CLIENT_CORE_INSECURE_TOKEN_STORE", "1");

        let messages = Arc::new(Mutex::new(Vec::new()));
        let count = Arc::new(AtomicUsize::new(0));
        let observer = RecordingObserver {
            messages: messages.clone(),
            count: count.clone(),
        };

        let client = NodClient::new(Box::new(observer), Box::new(UnusedSigner))
            .await
            .expect("runtime");
        client.start().await;

        // The pump runs on a detached task; give it a beat to drain the two
        // start-up messages.
        for _ in 0..50 {
            if count.load(Ordering::SeqCst) >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let seen = messages.lock().unwrap().clone();
        assert!(
            seen.iter().any(|m| m.contains("\"kind\":\"ready\"")),
            "expected a ready event, saw {seen:?}"
        );
        assert!(
            seen.iter().any(|m| m.contains("\"kind\":\"state\"")),
            "expected a state event, saw {seen:?}"
        );

        let response = client
            .request(r#"{"id":"r1","method":"state","params":{}}"#.to_string())
            .await;
        assert!(response.contains("\"ok\":true"), "state rpc failed: {response}");
        assert!(response.contains("\"id\":\"r1\""));

        let bad = client.request("not json".to_string()).await;
        assert!(bad.contains("\"ok\":false"));
        assert!(bad.contains("invalid rpc request json"));

        std::fs::remove_dir_all(&temp).ok();
    }
}
