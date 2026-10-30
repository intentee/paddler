use tokio::sync::mpsc::UnboundedSender;

use crate::send_startup_signal::send_startup_signal;
use crate::startup_signal_delivery::StartupSignalDelivery;

pub fn run_scheduler_unless_startup_abandoned<TSignal>(
    scheduler_startup_tx: &UnboundedSender<TSignal>,
    signal: TSignal,
    run_scheduler: impl FnOnce(),
) {
    match send_startup_signal(scheduler_startup_tx, signal) {
        StartupSignalDelivery::Abandoned => {}
        StartupSignalDelivery::Delivered => run_scheduler(),
    }
}
