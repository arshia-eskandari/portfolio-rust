use std::net::SocketAddr;

use portfolio::app::{AppState, build_router};
use portfolio::config::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    if let Err(err) = run().await {
        tracing::error!(error = %err, "fatal startup error");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    // One process-wide MongoDB client for all OxiMod models. The driver
    // connects lazily, so startup does not require the database to be up.
    oximod::OxiClient::init_global(config.database_url.clone()).await?;
    tracing::info!(
        db = portfolio::config::DB_NAME,
        "MongoDB client initialized"
    );

    // AWS SDK: region/credentials come from the environment
    // (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    let aws_config = aws_config::from_env()
        .region(aws_config::Region::new(config.aws_region.clone()))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let state = AppState::new(config, s3);
    let router = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "portfolio server listening");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
