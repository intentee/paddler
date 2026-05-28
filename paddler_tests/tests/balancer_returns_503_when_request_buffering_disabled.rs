#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler::balancer::inference_client::Message;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_cluster::start_cluster;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_503_when_request_buffering_disabled() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 0,
        ..ClusterParams::default()
    })
    .await?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "buffer-disabled-agent".to_owned(),
        slot_count: 2,
    });

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

    cluster.shutdown().await?;

    Ok(())
}
