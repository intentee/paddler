use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_reports_a_socket_closed_before_any_answer() {
    let cluster = start_cluster(cluster_without_agents_serving(
        InferenceMode::TextGeneration,
    ))
    .await
    .expect("a balancer serving text generation must start");
    let mut raw_inference_socket =
        RawInferenceSocket::connect(cluster.balancer.addresses.inference)
            .await
            .expect("the raw socket must reach the inference socket");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    assert!(matches!(
        raw_inference_socket.next_answer().await,
        Err(ClusterHarnessError::InferenceSocketClosedBeforeAnyAnswer)
    ));
}
