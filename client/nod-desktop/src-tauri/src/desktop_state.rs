use std::sync::Arc;

use nod_client_core::NodClientRuntime;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct DesktopState {
    pub(crate) runtime: Arc<Mutex<NodClientRuntime>>,
}

impl DesktopState {
    pub(crate) fn new(runtime: Arc<Mutex<NodClientRuntime>>) -> Self {
        Self { runtime }
    }
}
