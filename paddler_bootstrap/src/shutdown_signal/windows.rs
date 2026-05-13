use anyhow::Context as _;
use anyhow::Result;
use log::info;
use tokio::signal::windows::CtrlBreak;
use tokio::signal::windows::CtrlC;
use tokio::signal::windows::CtrlClose;
use tokio::signal::windows::CtrlShutdown;
use tokio::signal::windows::ctrl_break;
use tokio::signal::windows::ctrl_c;
use tokio::signal::windows::ctrl_close;
use tokio::signal::windows::ctrl_shutdown;

pub struct ShutdownSignals {
    ctrl_c: CtrlC,
    ctrl_break: CtrlBreak,
    ctrl_close: CtrlClose,
    ctrl_shutdown: CtrlShutdown,
}

impl ShutdownSignals {
    pub async fn wait(mut self) -> Result<()> {
        tokio::select! {
            _ = self.ctrl_c.recv() => info!("Received Ctrl+C"),
            _ = self.ctrl_break.recv() => info!("Received Ctrl+Break"),
            _ = self.ctrl_close.recv() => info!("Received console close"),
            _ = self.ctrl_shutdown.recv() => info!("Received system shutdown"),
        }

        Ok(())
    }
}

pub fn register_shutdown_signals() -> Result<ShutdownSignals> {
    let ctrl_c = ctrl_c().context("failed to listen for Ctrl+C")?;
    let ctrl_break = ctrl_break().context("failed to listen for Ctrl+Break")?;
    let ctrl_close = ctrl_close().context("failed to listen for console close")?;
    let ctrl_shutdown = ctrl_shutdown().context("failed to listen for system shutdown")?;

    Ok(ShutdownSignals {
        ctrl_c,
        ctrl_break,
        ctrl_close,
        ctrl_shutdown,
    })
}
