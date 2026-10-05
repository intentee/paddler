use anyhow::Result;
use anyhow::anyhow;
use log::error;
use tokio::sync::oneshot;

pub fn send_startup_signal<TSignal>(
    startup_signal_tx: oneshot::Sender<TSignal>,
    signal: TSignal,
    failure_message: String,
) -> Result<()> {
    if startup_signal_tx.send(signal).is_err() {
        error!("{failure_message}");

        return Err(anyhow!(failure_message));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use tokio::sync::oneshot;

    use super::send_startup_signal;

    #[test]
    fn delivers_the_signal_to_a_live_receiver() {
        let (startup_signal_tx, startup_signal_rx) = oneshot::channel::<()>();

        send_startup_signal(startup_signal_tx, (), "must not fail".to_owned())
            .expect("a live receiver must accept the signal");

        assert_eq!(startup_signal_rx.blocking_recv(), Ok(()));
    }

    #[test]
    fn fails_when_the_receiver_was_dropped() {
        log::set_max_level(LevelFilter::Trace);

        let (startup_signal_tx, startup_signal_rx) = oneshot::channel::<()>();

        drop(startup_signal_rx);

        let error = send_startup_signal(startup_signal_tx, (), "receiver is gone".to_owned())
            .expect_err("a dropped receiver must fail the startup signal");

        assert_eq!(error.to_string(), "receiver is gone");
    }
}
