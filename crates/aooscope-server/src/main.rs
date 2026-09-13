use aooscope_config::AppPaths;
use aooscope_server::{AppState, app};
use std::{env, net::SocketAddr, path::PathBuf};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let config_root = env::var_os("AOOSCOPE_CONFIG_DIR_IN_CONTAINER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/app/cfg"));
    let bind: SocketAddr = env::var("AOOSCOPE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8765".into())
        .parse()?;
    let state = AppState::new(AppPaths::new(config_root));
    let _media_poller = state.spawn_media_polling(std::time::Duration::from_secs(5));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "AooScope server listening");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
