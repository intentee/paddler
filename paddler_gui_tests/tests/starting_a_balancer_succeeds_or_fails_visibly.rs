use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use iced::futures::StreamExt as _;
use iced::futures::channel::mpsc;
use paddler_gui::drive_balancer_stream::drive_balancer_stream;
use paddler_gui::message::Message;
use paddler_gui_tests::bind_addresses::BindAddresses;
use paddler_gui_tests::bind_ephemeral_socket::bind_ephemeral_socket;
use paddler_gui_tests::make_balancer_runner_params::make_balancer_runner_params;
use tokio_util::sync::CancellationToken;

const BALANCER_STREAM_TIMEOUT: Duration = Duration::from_secs(30);

const INVALID_BIND_ADDR: &str = "192.0.2.1:1";

#[tokio::test]
async fn an_invalid_bind_address_tells_the_user_the_balancer_failed_to_start() -> Result<()> {
    let invalid: SocketAddr = INVALID_BIND_ADDR.parse()?;
    let cancellation_token = CancellationToken::new();
    let params = make_balancer_runner_params(
        BindAddresses {
            inference_addr: invalid,
            management_addr: invalid,
        },
        cancellation_token,
    );

    let (output, mut receiver) = mpsc::channel::<Message>(8);
    let driver = tokio::spawn(drive_balancer_stream(params, output));

    let collect = async {
        let mut observed_failed = false;
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::BalancerFailed(_)) {
                observed_failed = true;
                break;
            }
        }
        observed_failed
    };

    let observed_failed = tokio::time::timeout(BALANCER_STREAM_TIMEOUT, collect).await?;

    tokio::time::timeout(BALANCER_STREAM_TIMEOUT, driver).await??;

    if !observed_failed {
        bail!("expected BalancerFailed message for an invalid bind address");
    }

    Ok(())
}

#[tokio::test]
#[expect(
    clippy::needless_continue,
    reason = "explicit continue documents the no-op branch for readability"
)]
async fn a_running_balancer_reports_started_and_then_stopped_when_the_user_cancels() -> Result<()> {
    let cancellation_token = CancellationToken::new();
    let params = make_balancer_runner_params(
        BindAddresses {
            inference_addr: bind_ephemeral_socket()?,
            management_addr: bind_ephemeral_socket()?,
        },
        cancellation_token.clone(),
    );

    let (output, mut receiver) = mpsc::channel::<Message>(16);
    let driver = tokio::spawn(drive_balancer_stream(params, output));

    let mut observed_started = false;
    let mut observed_stopped = false;

    let collect = async {
        while let Some(message) = receiver.next().await {
            match message {
                Message::BalancerStarted => {
                    observed_started = true;
                    cancellation_token.cancel();
                }
                Message::BalancerStopped => {
                    observed_stopped = true;
                    break;
                }
                Message::BalancerFailed(error) => {
                    bail!("unexpected BalancerFailed during happy path: {error}");
                }
                _ => continue,
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::time::timeout(BALANCER_STREAM_TIMEOUT, collect).await??;

    tokio::time::timeout(BALANCER_STREAM_TIMEOUT, driver).await??;

    if !observed_started {
        bail!("expected BalancerStarted to be observed");
    }
    if !observed_stopped {
        bail!("expected BalancerStopped to be observed after cancellation");
    }

    Ok(())
}

#[tokio::test]
async fn when_the_ui_goes_away_the_balancer_stream_exits_without_panicking() -> Result<()> {
    let cancellation_token = CancellationToken::new();
    let params = make_balancer_runner_params(
        BindAddresses {
            inference_addr: bind_ephemeral_socket()?,
            management_addr: bind_ephemeral_socket()?,
        },
        cancellation_token.clone(),
    );

    let (output, receiver) = mpsc::channel::<Message>(1);
    drop(receiver);

    let driver = tokio::spawn(drive_balancer_stream(params, output));

    cancellation_token.cancel();

    tokio::time::timeout(BALANCER_STREAM_TIMEOUT, driver).await??;

    Ok(())
}

#[tokio::test]
async fn a_failed_start_with_a_disconnected_ui_logs_the_error_and_exits_cleanly() -> Result<()> {
    let invalid: SocketAddr = INVALID_BIND_ADDR.parse()?;
    let cancellation_token = CancellationToken::new();
    let params = make_balancer_runner_params(
        BindAddresses {
            inference_addr: invalid,
            management_addr: invalid,
        },
        cancellation_token,
    );

    let (output, receiver) = mpsc::channel::<Message>(1);
    drop(receiver);

    let driver = tokio::spawn(drive_balancer_stream(params, output));

    tokio::time::timeout(BALANCER_STREAM_TIMEOUT, driver).await??;

    Ok(())
}
