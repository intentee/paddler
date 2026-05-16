use anyhow::Result;
use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler_bootstrap::shutdown_signal::ShutdownSignals;

use crate::message::Message;

pub async fn drive_shutdown_signal_stream(
    signals: Result<ShutdownSignals>,
    mut output: Sender<Message>,
) {
    let shutdown_signals = match signals {
        Ok(shutdown_signals) => shutdown_signals,
        Err(error) => {
            log::error!("failed to register shutdown signal handlers: {error}");

            return;
        }
    };

    if let Err(error) = shutdown_signals.wait().await {
        log::error!("shutdown signal listener failed: {error}");

        return;
    }

    if let Err(err) = output.send(Message::Quit).await {
        log::warn!("Failed to deliver Quit message to iced runtime (receiver dropped): {err}");
    }
}
