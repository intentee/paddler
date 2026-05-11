use anyhow::Context as _;
use anyhow::Result;
use log::info;
use tokio::signal::unix::Signal;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;

pub struct ShutdownSignals {
    sigterm: Signal,
    sigint: Signal,
    sighup: Signal,
}

impl ShutdownSignals {
    pub async fn wait(mut self) -> Result<()> {
        tokio::select! {
            _ = self.sigterm.recv() => info!("Received SIGTERM"),
            _ = self.sigint.recv() => info!("Received SIGINT"),
            _ = self.sighup.recv() => info!("Received SIGHUP"),
        }

        Ok(())
    }
}

pub fn register_shutdown_signals() -> Result<ShutdownSignals> {
    let sigterm = signal(SignalKind::terminate()).context("failed to listen for SIGTERM")?;
    let sigint = signal(SignalKind::interrupt()).context("failed to listen for SIGINT")?;
    let sighup = signal(SignalKind::hangup()).context("failed to listen for SIGHUP")?;

    Ok(ShutdownSignals {
        sigterm,
        sigint,
        sighup,
    })
}
