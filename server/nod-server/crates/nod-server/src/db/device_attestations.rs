use sqlx::SqlitePool;

use super::now_string;
use crate::{error::ApiError, models::DeviceAttestationRecord};

pub async fn record_device_attestation(
    pool: &SqlitePool,
    device_id: &str,
    record: DeviceAttestationRecord,
) -> Result<(), ApiError> {
    let now = now_string();
    sqlx::query(
        r#"
        INSERT INTO device_attestations (
            device_id, provider, status, key_id, team_id, bundle_id, environment,
            public_key, counter, receipt_hash, verified_at, failure_reason, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(device_id, provider) DO UPDATE SET
            status = excluded.status,
            key_id = excluded.key_id,
            team_id = excluded.team_id,
            bundle_id = excluded.bundle_id,
            environment = excluded.environment,
            public_key = excluded.public_key,
            counter = excluded.counter,
            receipt_hash = excluded.receipt_hash,
            verified_at = excluded.verified_at,
            failure_reason = excluded.failure_reason,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(device_id)
    .bind(record.provider)
    .bind(record.status.as_str())
    .bind(record.key_id)
    .bind(record.team_id)
    .bind(record.bundle_id)
    .bind(record.environment)
    .bind(record.public_key)
    .bind(record.counter)
    .bind(record.receipt_hash)
    .bind(
        record
            .verified_at
            .map(|verified_at| verified_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
    )
    .bind(record.failure_reason)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}
