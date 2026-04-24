use anyhow::Context as _;
use anyhow::Result;
use log::info;
use tokio::signal::windows::ctrl_break;
use tokio::signal::windows::ctrl_c;
use tokio::signal::windows::ctrl_close;
use tokio::signal::windows::ctrl_shutdown;

pub async fn wait_for_shutdown_signal() -> Result<()> {
    let mut ctrl_c = ctrl_c().context("failed to listen for Ctrl+C")?;
    let mut ctrl_break = ctrl_break().context("failed to listen for Ctrl+Break")?;
    let mut ctrl_close = ctrl_close().context("failed to listen for console close")?;
    let mut ctrl_shutdown = ctrl_shutdown().context("failed to listen for system shutdown")?;

    tokio::select! {
        _ = ctrl_c.recv() => info!("Received Ctrl+C"),
        _ = ctrl_break.recv() => info!("Received Ctrl+Break"),
        _ = ctrl_close.recv() => info!("Received console close"),
        _ = ctrl_shutdown.recv() => info!("Received system shutdown"),
    }

    Ok(())
}
