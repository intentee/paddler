use paddler_client::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::inference_socket_round_trip::inference_socket_round_trip;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_late_cluster_inference_mode_subscriber_sees_the_current_mode() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");

    inference_socket_round_trip(&cluster.client_inference)
        .await
        .expect("the inference socket must answer before anyone subscribes");

    assert_eq!(
        *cluster
            .client_inference
            .subscribe_to_cluster_inference_mode()
            .borrow(),
        ReportedClusterInferenceMode::Reported(InferenceMode::Embeddings)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
