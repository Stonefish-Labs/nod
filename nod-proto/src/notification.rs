//! Notification delivery hints carried on requests and sync envelopes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryMode {
    Push,
    #[default]
    Websocket,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDelivery {
    #[serde(default)]
    pub mode: NotificationDeliveryMode,
}
