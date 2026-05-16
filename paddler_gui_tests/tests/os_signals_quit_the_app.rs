use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use iced::futures::StreamExt as _;
use iced::futures::channel::mpsc;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_bootstrap::shutdown_signal::register_shutdown_signals;
use paddler_gui::drive_shutdown_signal_stream::drive_shutdown_signal_stream;
use paddler_gui::message::Message;
use serial_test::serial;

const SIGNAL_DELIVERY_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
#[serial]
async fn if_signal_registration_fails_the_app_logs_and_keeps_running_without_quitting() -> Result<()>
{
    let (output, mut receiver) = mpsc::channel::<Message>(1);

    drive_shutdown_signal_stream(
        Err(anyhow::anyhow!("simulated registration failure")),
        output,
    )
    .await;

    match receiver.next().await {
        None => Ok(()),
        Some(message) => bail!("expected no message after registration failure, got {message:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn sigterm_delivered_to_the_process_makes_the_app_quit() -> Result<()> {
    let signals = register_shutdown_signals()?;
    let (output, mut receiver) = mpsc::channel::<Message>(1);

    let driver = tokio::spawn(drive_shutdown_signal_stream(Ok(signals), output));

    // Give the signal handler a moment to register before we raise the signal.
    tokio::task::yield_now().await;
    kill(Pid::this(), Signal::SIGTERM)?;

    let received = tokio::time::timeout(SIGNAL_DELIVERY_TIMEOUT, receiver.next()).await?;

    match received {
        Some(Message::Quit) => {}
        other => bail!("expected Message::Quit, got {other:?}"),
    }

    tokio::time::timeout(SIGNAL_DELIVERY_TIMEOUT, driver).await??;

    Ok(())
}
