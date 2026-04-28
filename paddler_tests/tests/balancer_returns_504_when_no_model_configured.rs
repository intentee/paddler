#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::inference_client::Message;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_no_model_configured() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

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
            assert_eq!(envelope.error.code, 504);
        }
        Message::Response(_) => {
            anyhow::bail!("expected an error response, got success");
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
