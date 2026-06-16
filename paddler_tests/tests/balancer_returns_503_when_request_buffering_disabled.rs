#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_503_when_request_buffering_disabled() -> Result<()> {
    let mut cluster = Cluster::start(
        &InProcessClusterBackend::new(BalancerServiceConfig {
            buffered_request_timeout: Duration::from_millis(50),
            max_buffered_requests: 0,
            ..Default::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "buffer-disabled-agent".to_owned(),
            slot_count: 2,
        })
        .await?;

    let mut stream = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
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
