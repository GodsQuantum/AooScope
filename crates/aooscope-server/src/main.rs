use aooscope_config::AppPaths;
use aooscope_display::{AoostarDisplayDriver, SimulatedDisplayDriver};
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
    let state = if env::var("AOOSCOPE_DISPLAY_MODE").as_deref() == Ok("real") {
        let device = state.device.clone();
        state.with_display_driver(AoostarDisplayDriver::open(device)?)
    } else {
        state.with_display_driver(SimulatedDisplayDriver::new())
    };
    state.display.power_on().await?;
    let display_runtime = state.start_display_runtime();
    let _media_poller = state.spawn_media_polling(std::time::Duration::from_secs(5));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "AooScope server listening");
    axum::serve(listener, app(state.clone()))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    display_runtime.abort();
    state.display.power_off().await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("install ctrl-c handler");
}
