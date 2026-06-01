use std::sync::Arc;

use anyhow::Context;
use nod_apns_relay::{router, tls::MtlsListener, AppleApnsProvider, Config, RelayPolicy};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nod_apns_relay=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load().context("failed to load configuration")?;
    let delivery = Arc::new(
        AppleApnsProvider::new(config.apns.clone())
            .context("failed to initialize Apple APNs delivery")?,
    );
    let app = router(delivery, RelayPolicy::new(config.apns.bundle_id.clone()))
        .layer(TraceLayer::new_for_http());
    let listener = MtlsListener::bind(config.bind, &config.tls).await?;

    tracing::info!(addr = %config.bind, "nod APNs relay listening with mTLS");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
