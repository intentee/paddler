use std::mem::discriminant;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::raw_inference_socket::RawInferenceSocket;

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_reports_a_socket_closed_before_a_pong() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
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
