#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures_util::StreamExt as _;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_closes_management_websocket_on_sigterm() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let management_addr = cluster.addresses.management;
    let ws_url = format!("ws://{management_addr}/api/v1/agent_socket/test_agent_shutdown_probe");
    let (mut ws_stream, _response) = connect_async(ws_url).await?;

    let first_frame = ws_stream
        .next()
        .await
        .ok_or_else(|| anyhow!("WebSocket closed before yielding the version notification"))??;

    match first_frame {
        Message::Text(_) => {}
        other => bail!("expected initial Text frame, got {other:?}"),
    }

    let observe_close = tokio::spawn(async move {
        while let Some(item) = ws_stream.next().await {
            match item {
                Ok(Message::Close(_)) => return Ok::<bool, anyhow::Error>(true),
                Ok(_) => {}
                Err(_) => break,
            }
        }

        Ok(false)
    });

    cluster.shutdown().await?;

    let saw_close_frame = observe_close.await??;

    assert!(
        saw_close_frame,
        "WebSocket client must observe a Close frame after the balancer is SIGTERMed"
    );

    Ok(())
}
