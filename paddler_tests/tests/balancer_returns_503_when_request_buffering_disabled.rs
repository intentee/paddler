#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::inference_client::Message;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_503_when_request_buffering_disabled() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 0,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let mut agent_child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("buffer-disabled-agent".to_owned()),
        slots: 2,
    })?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let message = stream
        .next()
        .await
        .context("inference stream must yield a message")??;

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 503);
        }
        Message::Response(_) => {
            anyhow::bail!("expected buffer overflow error, got success");
        }
    }

    agent_child.start_kill()?;
    agent_child.wait().await?;

    cluster.shutdown().await?;

    Ok(())
}
