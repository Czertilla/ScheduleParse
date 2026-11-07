mod config;
mod fetch;
mod ical;
mod schedule;
mod server;
mod state;
mod worker;
mod xlsx;

use std::time::Duration;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env()?;

    let client = reqwest::Client::builder()
        .timeout(config.http_timeout)
        .user_agent("SkeduleParse/0.1 (+calendar sync worker)")
        .build()?;

    let shared_state = state::new_shared_state();

    let worker_handle = tokio::spawn(worker::run(config.clone(), client, shared_state.clone()));

    let router = server::build_router(&config.calendar_path, shared_state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, path = %config.calendar_path, "listening");

    tokio::select! {
        result = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()) => {
            result?;
        }
        _ = worker_handle => {
            tracing::error!("background worker task exited unexpectedly");
        }
    }

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
    // Avoid racing the log line with process exit on fast shutdowns.
    tokio::time::sleep(Duration::from_millis(10)).await;
}
