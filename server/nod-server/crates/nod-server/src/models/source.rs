use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub emoji: String,
    #[serde(default = "default_subscribed")]
    pub subscribed: bool,
    pub created_at: DateTime<Utc>,
}

fn default_subscribed() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSourceRequest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_emoji")]
    pub emoji: String,
}

fn default_emoji() -> String {
    "🔔".to_string()
}
