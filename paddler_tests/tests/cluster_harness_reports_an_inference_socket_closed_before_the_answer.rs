use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

const UNANSWERED_REQUEST_ID: &str = "request-the-balancer-never-received";

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_an_inference_socket_closed_before_the_answer() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let answer = socket.next_answer_to(UNANSWERED_REQUEST_ID).await;

    assert!(matches!(
        answer,
        Err(ClusterHarnessError::InferenceSocketClosedBeforeAnswer { request_id })
            if request_id == UNANSWERED_REQUEST_ID
    ));
}
