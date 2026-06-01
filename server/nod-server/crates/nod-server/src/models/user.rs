use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUser {
    pub id: String,
    pub name: String,
    pub device_count: i64,
    pub subscribed_source_count: i64,
    pub subscribed_source_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrollmentCodeResponse {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEnrollmentCodeRequest {
    #[serde(default)]
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub subscribed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserSubscriptionUpdate {
    pub source_id: String,
    pub subscribed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserSubscriptionsRequest {
    #[serde(default)]
    pub updates: Vec<AdminUserSubscriptionUpdate>,
}
