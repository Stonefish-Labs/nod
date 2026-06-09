use std::sync::Arc;

use reqwest::Client;
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};

use crate::{
    audit::AuditLogger,
    config::Config,
    db,
    models::NotificationDelivery,
    push::{build_push_registry, notification_delivery_for_route, PushRegistry, PushRoute},
    sync::{self, SyncSender},
};

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Arc<Config>,
    pub(crate) pool: SqlitePool,
    pub(crate) audit: AuditLogger,
    pub(crate) sync: SyncSender,
    pub(crate) push: PushRegistry,
    pub(crate) notification_delivery: NotificationDelivery,
    pub(crate) push_route: Option<PushRoute>,
    pub(crate) http: Client,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let pool = db::connect(&config).await?;
        let audit = AuditLogger::new(config.data_dir.clone()).await?;
        let built_push = build_push_registry(&config.notifications)?;
        let notification_delivery = notification_delivery_for_route(built_push.route);
        let state = Self {
            config: Arc::new(config),
            pool,
            audit,
            sync: sync::sender(),
            push: built_push.registry,
            notification_delivery,
            push_route: built_push.route,
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
        };
        state.spawn_maintenance();
        Ok(state)
    }

    fn spawn_maintenance(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(30));
            loop {
                tick.tick().await;
                match db::expire_due_requests(&state.pool).await {
                    Ok(expired_requests) => {
                        for request in expired_requests {
                            let _ = state.sync.send(sync::request("expired", &request));
                            state.audit.record("request.expired", &request).await;
                        }
                    }
                    Err(err) => tracing::error!(error = %err, "failed to expire due requests"),
                }

                match db::prune_retention(&state.pool, state.config.retention_days).await {
                    Ok(count) if count > 0 => tracing::info!(count, "pruned old requests"),
                    Ok(_) => {}
                    Err(err) => tracing::error!(error = %err, "failed to prune old requests"),
                }
            }
        });
    }
}
