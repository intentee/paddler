
use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler::balancer::inference_client::Message;
use paddler::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_no_model_configured() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: Vec::new(),
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
