#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::pick_balancer_addresses::pick_balancer_addresses;
use serial_test::file_serial;
use tempfile::NamedTempFile;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

const GRACEFUL_EXIT_DEADLINE: Duration = Duration::from_secs(5);
const WS_CLOSE_FRAME_DEADLINE: Duration = Duration::from_secs(5);

#[tokio::test]
#[file_serial]
async fn balancer_closes_websocket_and_exits_on_sigterm() -> Result<()> {
    let state_db = NamedTempFile::new()?;
    let state_db_path = state_db
        .path()
        .to_str()
        .ok_or_else(|| anyhow!("temp file path is not valid UTF-8"))?;
    let state_db_url = format!("file://{state_db_path}");

    let addresses = pick_balancer_addresses()?;
    let management_addr = addresses.management.clone();

    let mut balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(5),
        compat_openai_addr: addresses.compat_openai,
        inference_addr: addresses.inference,
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management,
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 16,
        state_database_url: state_db_url,
    })
    .await?;

    let ws_url = format!("ws://{management_addr}/api/v1/agent_socket/test_agent_shutdown_probe");
    let (mut ws_stream, _response) = connect_async(ws_url).await?;

    let first_frame = ws_stream
        .next()
        .await
        .ok_or_else(|| anyhow!("WebSocket closed before receiving the version notification"))??;

    match first_frame {
        Message::Text(_) => {}
        other => bail!("expected initial Text frame with Version notification, got {other:?}"),
    }

    let pid_raw = balancer
        .pid()
        .ok_or_else(|| anyhow!("managed balancer has no child pid"))?;
    #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
    let pid = Pid::from_raw(pid_raw as i32);

    kill(pid, Signal::SIGTERM)?;

    let close_deadline = Instant::now() + WS_CLOSE_FRAME_DEADLINE;
    let mut saw_close_frame = false;

    while Instant::now() < close_deadline {
        match ws_stream.next().await {
            Some(Ok(Message::Close(_))) => {
                saw_close_frame = true;
                break;
            }
            Some(Ok(_)) => {}
            Some(Err(_)) | None => break,
        }
    }

    let _ = ws_stream.send(Message::Close(None)).await;

    let exit_start = Instant::now();
    let exit_status = tokio::time::timeout(GRACEFUL_EXIT_DEADLINE, balancer.wait_for_exit())
        .await
        .map_err(|_| {
            anyhow!(
                "balancer did not exit within {GRACEFUL_EXIT_DEADLINE:?} after SIGTERM; \
                 WebSocket close was {}received",
                if saw_close_frame { "" } else { "not " }
            )
        })??;

    assert!(
        exit_status.success() || exit_status.code().is_some(),
        "balancer terminated abnormally: {exit_status:?}"
    );

    assert!(
        saw_close_frame,
        "WebSocket client did not receive a Close frame from the balancer; \
         elapsed = {:?}",
        exit_start.elapsed()
    );

    Ok(())
}
