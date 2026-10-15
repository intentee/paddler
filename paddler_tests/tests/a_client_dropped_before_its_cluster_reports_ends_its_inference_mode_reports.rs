use std::num::NonZeroUsize;

use tokio::sync::watch::channel;

use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::next_reported_cluster_inference_mode::next_reported_cluster_inference_mode;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_client_dropped_before_its_cluster_reports_ends_its_inference_mode_reports() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");
    let client_inference = ClientInference::new(ClientInferenceParams {
        inference_socket_pool_size: NonZeroUsize::MIN,
        url: cluster
            .balancer
            .inference_base_url()
            .expect("the balancer must expose its inference address"),
    });
    let mut cluster_inference_mode_rx = client_inference.subscribe_to_cluster_inference_mode();

    drop(client_inference);

    let (closed_channel_tx, mut closed_channel_rx) = channel(());

    drop(closed_channel_tx);

    assert_eq!(
        next_reported_cluster_inference_mode(&mut cluster_inference_mode_rx)
            .await
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            closed_channel_rx
                .changed()
                .await
                .expect_err("a channel without its sender must be closed")
                .to_string()
        )
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
