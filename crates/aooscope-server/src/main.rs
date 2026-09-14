use aooscope_config::AppPaths;
use aooscope_display::{AoostarDisplayDriver, SimulatedDisplayDriver};
use aooscope_server::{AppState, app, bootstrap};
use std::{
    env,
    ffi::OsString,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};
use tracing_subscriber::EnvFilter;

fn config_root(container: Option<OsString>, configured: Option<OsString>) -> PathBuf {
    container
        .or(configured)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/app/cfg"))
}

fn device_path(configured: Option<OsString>) -> PathBuf {
    configured
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/dev/ttyACM0"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("health") {
        return health_check().map_err(Into::into);
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let config_root = config_root(
        env::var_os("AOOSCOPE_CONFIG_DIR_IN_CONTAINER"),
        env::var_os("AOOSCOPE_CONFIG_DIR"),
    );
    let device = device_path(env::var_os("AOOSCOPE_DEVICE"));
    let bind: SocketAddr = env::var("AOOSCOPE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8765".into())
        .parse()?;
    bootstrap(&AppPaths::new(&config_root))?;
    let state = AppState::new(AppPaths::new(config_root)).with_device(device.clone());
    let state = if env::var("AOOSCOPE_DISPLAY_MODE").as_deref() == Ok("real") {
        state.with_display_driver(AoostarDisplayDriver::open(device)?)
    } else {
        state.with_display_driver(SimulatedDisplayDriver::new())
    };
    state.display.power_on().await?;
    let display_runtime = state.start_display_runtime();
    let _media_poller = state.spawn_media_polling(std::time::Duration::from_secs(5));
    let _telemetry_poller = state.spawn_telemetry_polling(std::time::Duration::from_secs(5));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "AooScope server listening");
    axum::serve(listener, app(state.clone()))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    display_runtime.abort();
    state.display.power_off().await?;
    Ok(())
}

fn health_check() -> Result<(), String> {
    let bind = env::var("AOOSCOPE_BIND").unwrap_or_else(|_| "127.0.0.1:8765".into());
    let address = bind
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid AOOSCOPE_BIND: {error}"))?;
    let address = SocketAddr::new(
        if address.ip().is_unspecified() {
            "127.0.0.1".parse().unwrap()
        } else {
            address.ip()
        },
        address.port(),
    );
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .map_err(|error| format!("health connection failed: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;
    if response.starts_with("HTTP/1.1 200 ") || response.starts_with("HTTP/1.0 200 ") {
        Ok(())
    } else {
        Err(format!(
            "health endpoint returned {}",
            response.lines().next().unwrap_or("no response")
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::{config_root, device_path};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn config_root_prefers_container_then_configured_then_default() {
        assert_eq!(
            config_root(
                Some(OsString::from("/container")),
                Some(OsString::from("/configured"))
            ),
            PathBuf::from("/container")
        );
        assert_eq!(
            config_root(None, Some(OsString::from("/configured"))),
            PathBuf::from("/configured")
        );
        assert_eq!(config_root(None, None), PathBuf::from("/app/cfg"));
    }

    #[test]
    fn device_path_prefers_configured_then_default() {
        assert_eq!(
            device_path(Some(OsString::from("/dev/custom"))),
            PathBuf::from("/dev/custom")
        );
        assert_eq!(device_path(None), PathBuf::from("/dev/ttyACM0"));
    }
}
