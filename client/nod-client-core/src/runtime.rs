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
    models::{ClientState, Event, EventStatus},
    state::StateReducer,
    store::{PersistedConfig, Store},
};

pub use rpc::{
    ChannelParams, EnrollParams, NotificationPreferenceParams, RenameDeviceParams,
    RevokeDeviceParams, RpcRequest, RpcResponse, SelectEventParams, SelectServerParams,
    SetSubscriptionParams, SubmitActionParams,
};

const DEFAULT_NOTIFICATION_SOUND: &str = "default";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum NodClientEvent {
    Ready { state_path: String },
    State(Box<ClientState>),
    NotificationCandidate { event: Box<Event> },
    NotificationRemoved { event_id: String },
    SyncStatus { connected: bool },
    AuthRevoked {},
    ResyncRequired {},
    TransientError { message: String },
}

type Outbox = mpsc::Sender<NodClientEvent>;

pub struct NodClientRuntime {
    store: Store,
    persisted: Arc<Mutex<PersistedConfig>>,
    reducer: Arc<Mutex<StateReducer>>,
    tx: Outbox,
    sync_task: Option<JoinHandle<()>>,
}

impl NodClientRuntime {
    pub async fn new(tx: Outbox) -> Result<Self> {
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
        })
    }

    pub async fn emit_ready(&self) {
        self.emit_event(NodClientEvent::Ready {
            state_path: self.store.path().display().to_string(),
        })
        .await;
    }

    pub async fn state(&self) -> ClientState {
        self.reducer.lock().await.state.clone()
    }

    pub async fn emit_state(&self) {
        self.emit_event(NodClientEvent::State(Box::new(self.state().await)))
            .await;
    }

    async fn emit_notifications(&self, events: Vec<Event>) {
        for event in events {
            if event.status == EventStatus::Pending {
                self.emit_event(NodClientEvent::NotificationCandidate {
                    event: Box::new(event),
                })
                .await;
            }
        }
    }

    async fn emit_event(&self, event: NodClientEvent) {
        emit_to(&self.tx, event).await;
    }
}

async fn emit_to(tx: &Outbox, event: NodClientEvent) {
    let _ = tx.send(event).await;
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
