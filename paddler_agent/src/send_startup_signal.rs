use log::debug;
use tokio::sync::oneshot;

use crate::startup_signal_delivery::StartupSignalDelivery;

pub fn send_startup_signal<TSignal>(
    startup_signal_tx: oneshot::Sender<TSignal>,
    signal: TSignal,
) -> StartupSignalDelivery {
    match startup_signal_tx.send(signal) {
        Ok(()) => StartupSignalDelivery::Delivered,
        Err(_undelivered_signal) => {
            debug!("The arbiter abandoned the scheduler startup; stopping the scheduler thread");

            StartupSignalDelivery::Abandoned
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use tokio::sync::oneshot;

    use super::send_startup_signal;
    use crate::startup_signal_delivery::StartupSignalDelivery;

    #[test]
    fn delivers_the_signal_to_a_live_receiver() {
        let (startup_signal_tx, startup_signal_rx) = oneshot::channel::<()>();

        assert_eq!(
            discriminant(&send_startup_signal(startup_signal_tx, ())),
            discriminant(&StartupSignalDelivery::Delivered)
        );
        assert_eq!(startup_signal_rx.blocking_recv(), Ok(()));
    }

    #[test]
    fn reports_an_abandoned_startup_when_the_receiver_was_dropped() {
        let (startup_signal_tx, startup_signal_rx) = oneshot::channel::<()>();

        drop(startup_signal_rx);

        assert_eq!(
            discriminant(&send_startup_signal(startup_signal_tx, ())),
            discriminant(&StartupSignalDelivery::Abandoned)
        );
    }
}
