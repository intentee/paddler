use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::desired_state_serving::desired_state_serving;
use paddler_tests::inference_socket_round_trip::inference_socket_round_trip;
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

    let mut received_notifications = vec![
        cluster_inference_mode_rx
            .recv()
            .await
            .expect("the pool must report the inference mode the cluster serves"),
    ];

    for inference_mode in [InferenceMode::TextGeneration, InferenceMode::Decision] {
        cluster
            .client_management
            .put_balancer_desired_state(
                CancellationToken::new(),
                &desired_state_serving(inference_mode),
            )
            .await
            .expect("the balancer must apply the desired state");

        received_notifications.push(
            cluster_inference_mode_rx
                .recv()
                .await
                .expect("the pool must report the inference mode change"),
        );
    }

    assert_eq!(
        received_notifications,
        [
            Notification::ClusterInferenceMode(InferenceMode::Embeddings),
            Notification::ClusterInferenceMode(InferenceMode::TextGeneration),
            Notification::ClusterInferenceMode(InferenceMode::Decision),
        ]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
