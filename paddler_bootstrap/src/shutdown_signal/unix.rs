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

fn listen_for_signal(kind: SignalKind, description: &str) -> Result<Signal> {
    signal(kind).with_context(|| format!("failed to listen for {description}"))
}

fn register_signals(
    terminate_kind: SignalKind,
    interrupt_kind: SignalKind,
    hangup_kind: SignalKind,
) -> Result<ShutdownSignals> {
    Ok(ShutdownSignals {
        sigterm: listen_for_signal(terminate_kind, "SIGTERM")?,
        sigint: listen_for_signal(interrupt_kind, "SIGINT")?,
        sighup: listen_for_signal(hangup_kind, "SIGHUP")?,
    })
}

pub fn register_shutdown_signals() -> Result<ShutdownSignals> {
    register_signals(
        SignalKind::terminate(),
        SignalKind::interrupt(),
        SignalKind::hangup(),
    )
}

#[cfg(test)]
mod tests {
    use nix::sys::signal::Signal as UnixSignal;
    use nix::sys::signal::raise;
    use tokio::signal::unix::SignalKind;

    use super::register_shutdown_signals;
    use super::register_signals;

    #[tokio::test]
    async fn wait_returns_on_each_shutdown_signal() {
        for shutdown_signal in [UnixSignal::SIGTERM, UnixSignal::SIGINT, UnixSignal::SIGHUP] {
            let shutdown_signals = register_shutdown_signals().unwrap();

            raise(shutdown_signal).unwrap();

            shutdown_signals.wait().await.unwrap();
        }
    }

    #[tokio::test]
    async fn register_signals_errors_for_unregisterable_signal() {
        let unregisterable = SignalKind::from_raw(UnixSignal::SIGKILL as i32);

        assert!(
            register_signals(unregisterable, SignalKind::interrupt(), SignalKind::hangup())
                .is_err()
        );
        assert!(
            register_signals(SignalKind::terminate(), unregisterable, SignalKind::hangup())
                .is_err()
        );
        assert!(
            register_signals(SignalKind::terminate(), SignalKind::interrupt(), unregisterable)
                .is_err()
        );
    }
}
