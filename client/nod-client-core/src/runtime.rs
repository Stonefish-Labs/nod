mod rpc;
mod session;
mod sync;
mod workflows;

use std::sync::Arc;

use anyhow::Result;
use serde::Serialize;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

use crate::{
    models::{ClientState, Request, RequestStatus},
    signing::ForeignSigner,
    state::StateReducer,
    store::{PersistedConfig, Store},
};

pub use rpc::{
    ChannelParams, EnrollParams, NotificationPreferenceParams, RegisterPushTokenParams,
    RenameDeviceParams, RevokeDeviceParams, RpcRequest, RpcResponse, SelectRequestParams,
    SelectServerParams, SetSubscriptionParams, SubmitOptionParams,
};

const DEFAULT_NOTIFICATION_SOUND: &str = "default";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum NodClientMessage {
    Ready { state_path: String },
    State(Box<ClientState>),
    NotificationCandidate { request: Box<Request> },
    NotificationRemoved { request_id: String },
    SyncStatus { connected: bool },
    AuthRevoked {},
    ResyncRequired {},
    TransientError { message: String },
}

type Outbox = mpsc::Sender<NodClientMessage>;

/// Where the runtime gets device signing keys. The TUI + desktop use software
/// keys persisted in the `Store`; Apple injects a `Foreign` backend so signing
/// happens in the Secure Enclave and no private key is ever persisted by Rust.
pub enum SignerBackend {
    /// Software P-256 keys generated and stored by nod-client-core.
    Software,
    /// Host-owned hardware keys (Apple Secure Enclave) reached via a callback.
    Foreign(Arc<dyn ForeignSigner>),
}

pub struct NodClientRuntime {
    store: Store,
    persisted: Arc<Mutex<PersistedConfig>>,
    reducer: Arc<Mutex<StateReducer>>,
    tx: Outbox,
    sync_task: Option<JoinHandle<()>>,
    signer_backend: SignerBackend,
}

impl NodClientRuntime {
    /// Construct with the default software signing backend (TUI + desktop).
    pub async fn new(tx: Outbox) -> Result<Self> {
        Self::with_signer_backend(tx, SignerBackend::Software).await
    }

    /// Construct with an explicit signing backend. Apple passes
    /// `SignerBackend::Foreign(..)` so decisions are signed in the Secure
    /// Enclave instead of by a software key.
    pub async fn with_signer_backend(tx: Outbox, signer_backend: SignerBackend) -> Result<Self> {
        let store = Store::new()?;
        let mut persisted = store.load().await?;
        normalize_notification_sound(&mut persisted);

        let selected_server_id = selected_server_id_for(&persisted);
        persisted.selected_server_id = selected_server_id.clone();
        let reducer = StateReducer::new(
            persisted.servers.clone(),
            selected_server_id,
            persisted.notification_sound.clone(),
        );

        Ok(Self {
            store,
            persisted: Arc::new(Mutex::new(persisted)),
            reducer: Arc::new(Mutex::new(reducer)),
            tx,
            sync_task: None,
            signer_backend,
        })
    }

    pub(crate) fn signer_backend(&self) -> &SignerBackend {
        &self.signer_backend
    }

    pub async fn emit_ready(&self) {
        self.emit_message(NodClientMessage::Ready {
            state_path: self.store.path().display().to_string(),
        })
        .await;
    }

    pub async fn state(&self) -> ClientState {
        self.reducer.lock().await.state.clone()
    }

    pub async fn emit_state(&self) {
        self.emit_message(NodClientMessage::State(Box::new(self.state().await)))
            .await;
    }

    async fn emit_notifications(&self, requests: Vec<Request>) {
        for request in requests {
            if request.status == RequestStatus::Pending {
                self.emit_message(NodClientMessage::NotificationCandidate {
                    request: Box::new(request),
                })
                .await;
            }
        }
    }

    async fn emit_message(&self, message: NodClientMessage) {
        emit_to(&self.tx, message).await;
    }
}

async fn emit_to(tx: &Outbox, message: NodClientMessage) {
    let _ = tx.send(message).await;
}

fn normalize_notification_sound(config: &mut PersistedConfig) {
    if config.notification_sound.trim().is_empty() {
        config.notification_sound = DEFAULT_NOTIFICATION_SOUND.to_string();
    }
}

fn selected_server_id_for(config: &PersistedConfig) -> Option<String> {
    config
        .selected_server_id
        .clone()
        .filter(|id| config.servers.iter().any(|server| server.id == *id))
        .or_else(|| config.servers.first().map(|server| server.id.clone()))
}
