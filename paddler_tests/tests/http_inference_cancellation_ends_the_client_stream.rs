use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn http_inference_cancellation_ends_the_client_stream() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            cancellation_token.clone(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(16).unwrap(),
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the balancer must reach the expected buffered request count");

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled HTTP inference request must end its stream cleanly"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
