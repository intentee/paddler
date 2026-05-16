use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use iced::futures::StreamExt as _;
use iced::futures::channel::mpsc;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_gui::drive_agent_stream::drive_agent_stream;
use paddler_gui::message::Message;
use paddler_gui_tests::bind_ephemeral_socket::bind_ephemeral_socket;
use tokio_util::sync::CancellationToken;

const AGENT_STREAM_TIMEOUT: Duration = Duration::from_secs(10);

fn ephemeral_management_address() -> Result<String> {
    Ok(bind_ephemeral_socket()?.to_string())
}

#[tokio::test]
#[expect(
    clippy::needless_continue,
    reason = "explicit continue documents the no-op branch for readability"
)]
async fn cancellation_token_makes_the_agent_send_a_stopped_message_and_finish() -> Result<()> {
    let cancellation_token = CancellationToken::new();
    let params = AgentRunnerParams {
        agent_name: Some("test-agent".to_owned()),
        cancellation_token: cancellation_token.clone(),
        management_address: ephemeral_management_address()?,
        slots: 1,
    };

    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_agent_stream(params, output));

    cancellation_token.cancel();

    let mut observed_stopped = false;

    let collect = async {
        while let Some(message) = receiver.next().await {
            match message {
                Message::AgentStopped => {
                    observed_stopped = true;
                    break;
                }
                Message::AgentFailed(_) => break,
                _ => continue,
            }
        }
    };

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, collect).await?;

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    if !observed_stopped {
        bail!("expected AgentStopped to be observed before the stream finished");
    }

    Ok(())
}

#[tokio::test]
async fn when_the_ui_goes_away_the_agent_stream_exits_without_panicking() -> Result<()> {
    let cancellation_token = CancellationToken::new();
    let params = AgentRunnerParams {
        agent_name: None,
        cancellation_token: cancellation_token.clone(),
        management_address: ephemeral_management_address()?,
        slots: 1,
    };

    let (output, receiver) = mpsc::channel::<Message>(8);
    drop(receiver);

    let driver = tokio::spawn(drive_agent_stream(params, output));

    cancellation_token.cancel();

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    Ok(())
}
