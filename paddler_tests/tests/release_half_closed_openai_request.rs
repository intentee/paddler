use serde_json::Value;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_tests::start_cluster::start_cluster;

pub async fn release_half_closed_openai_request(path: &str, body: &Value) {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster
            .balancer
            .compat_openai_addr()
            .expect("the balancer must serve the OpenAI compatibility service"),
        path,
        body,
    )
    .await
    .expect("the client must send its OpenAI-compatible request");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the request must be buffered while no agent is available");
    client
        .half_close()
        .await
        .expect("the client must half-close its connection");
    cluster
        .wait_for_buffered_request_count(0)
        .await
        .expect("the balancer must release the buffered request once the client half-closes");

    drop(client);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
