use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryMode {
    Push,
    Websocket,
}

impl NotificationDeliveryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Websocket => "websocket",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationDelivery {
    pub mode: NotificationDeliveryMode,
}
