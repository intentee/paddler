use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn http_inference_cancellation_ends_the_client_stream() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            cancellation_token.clone(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 16,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .map_err(anyhow::Error::new)?;

    cluster.wait_for_buffered_request_count(1).await?;

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled HTTP inference request must end its stream cleanly"
    );

    cluster.shutdown().await?;

    Ok(())
}
