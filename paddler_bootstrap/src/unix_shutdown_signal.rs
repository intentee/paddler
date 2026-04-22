use anyhow::Context as _;
use anyhow::Result;
use log::info;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;

pub async fn wait_for_unix_shutdown_signal() -> Result<()> {
    let mut sigterm = signal(SignalKind::terminate()).context("failed to listen for SIGTERM")?;
    let mut sigint = signal(SignalKind::interrupt()).context("failed to listen for SIGINT")?;
    let mut sighup = signal(SignalKind::hangup()).context("failed to listen for SIGHUP")?;

    tokio::select! {
        _ = sigterm.recv() => info!("Received SIGTERM"),
        _ = sigint.recv() => info!("Received SIGINT"),
        _ = sighup.recv() => info!("Received SIGHUP"),
    }

    Ok(())
}
