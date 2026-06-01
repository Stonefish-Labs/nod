use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::Mutex};

#[derive(Clone)]
pub struct AuditLogger {
    inner: Arc<Mutex<tokio::fs::File>>,
}

impl AuditLogger {
    pub async fn new(data_dir: PathBuf) -> anyhow::Result<Self> {
        let audit_dir = data_dir.join("audit");
        tokio::fs::create_dir_all(&audit_dir).await?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(audit_dir.join("nod.audit.jsonl"))
            .await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn record<T: Serialize + ?Sized>(&self, kind: &str, payload: &T) {
        let line = json!({
            "at": Utc::now(),
            "kind": kind,
            "payload": payload,
        });
        let Ok(mut raw) = serde_json::to_vec(&line) else {
            tracing::error!(kind, "failed to encode audit entry");
            return;
        };
        raw.push(b'\n');
        let mut file = self.inner.lock().await;
        if let Err(err) = file.write_all(&raw).await {
            tracing::error!(kind, error = %err, "failed to write audit entry");
        }
    }
}
