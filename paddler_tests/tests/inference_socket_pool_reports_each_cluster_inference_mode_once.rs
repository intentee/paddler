use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::desired_state_serving::desired_state_serving;
use paddler_tests::inference_socket_round_trip::inference_socket_round_trip;
use paddler_tests::next_reported_cluster_inference_mode::next_reported_cluster_inference_mode;
use paddler_tests::start_cluster::start_cluster;

const INFERENCE_SOCKET_POOL_SIZE: NonZeroUsize = NonZeroUsize::new(2).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_pool_reports_each_cluster_inference_mode_once() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");
    let client_inference = ClientInference::new(ClientInferenceParams {
        inference_socket_pool_size: INFERENCE_SOCKET_POOL_SIZE,
        url: cluster
            .balancer
            .inference_base_url()
            .expect("the balancer must expose its inference address"),
    });
    let mut cluster_inference_mode_rx = client_inference.subscribe_to_cluster_inference_mode();

    for _pooled_socket_index in 0..INFERENCE_SOCKET_POOL_SIZE.get() {
        inference_socket_round_trip(&client_inference)
            .await
            .expect("every pooled inference socket must answer");
    }

    let mut reported_modes = vec![
        next_reported_cluster_inference_mode(&mut cluster_inference_mode_rx)
            .await
            .expect("the pool must report the inference mode the cluster serves"),
    ];

    assert!(
        !cluster_inference_mode_rx
            .has_changed()
            .expect("the pool must keep reporting the inference mode")
    );

    for inference_mode in [InferenceMode::TextGeneration, InferenceMode::Decision] {
        cluster
            .client_management
            .put_balancer_desired_state(
                CancellationToken::new(),
                &desired_state_serving(inference_mode),
            )
            .await
            .expect("the balancer must apply the desired state");

        reported_modes.push(
            next_reported_cluster_inference_mode(&mut cluster_inference_mode_rx)
                .await
                .expect("the pool must report the inference mode change"),
        );
    }

    assert_eq!(
        reported_modes,
        [
            ReportedClusterInferenceMode::Reported(InferenceMode::Embeddings),
            ReportedClusterInferenceMode::Reported(InferenceMode::TextGeneration),
            ReportedClusterInferenceMode::Reported(InferenceMode::Decision),
        ]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
