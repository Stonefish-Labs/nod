use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub default_priority: i64,
    pub privacy: String,
    #[serde(default = "default_subscribed")]
    pub subscribed: bool,
    pub created_at: DateTime<Utc>,
}

fn default_subscribed() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSourceRequest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub default_priority: Option<i64>,
    #[serde(default)]
    pub privacy: Option<String>,
}

fn default_icon() -> String {
    "bell".to_string()
}

fn default_color() -> String {
    "#3B82F6".to_string()
}
