use std::mem::discriminant;

use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_reports_a_socket_closed_before_a_pong() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut raw_inference_socket =
        RawInferenceSocket::connect(cluster.balancer.addresses.inference)
            .await
            .expect("the raw socket must reach the inference socket");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    assert_eq!(
        raw_inference_socket
            .next_pong()
            .await
            .err()
            .as_ref()
            .map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::InferenceSocketClosedBeforePong
        ))
    );
}
