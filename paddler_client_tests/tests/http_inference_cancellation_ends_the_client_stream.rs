use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client_tests::raw_prompt_about_the_capital_of_france::raw_prompt_about_the_capital_of_france;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn http_inference_cancellation_ends_the_client_stream() {
    let mut cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            cancellation_token.clone(),
            &raw_prompt_about_the_capital_of_france(
                NonZeroU32::new(16).expect("sixteen tokens must be a positive count"),
            ),
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
