use std::num::NonZeroU32;

use serde_json::to_value;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_messaging::api_path::ApiPath;
use paddler_request_params_fixture::greeting_conversation::greeting_conversation;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_conversation_client_releases_the_buffered_request() {
    let mut cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");

    let inference_addr = cluster.balancer.addresses.inference;
    let params = greeting_conversation(NonZeroU32::new(2048).unwrap());
    let mut client = HalfClosedClient::post_json_then_half_close(
        inference_addr,
        ApiPath::continue_from_conversation_history(),
        &to_value(&params).expect("the request parameters must serialize"),
    )
    .await
    .expect("the half-closed request must be sent");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the request must be buffered while no agent is available");

    client
        .half_close()
        .await
        .expect("the request must be half-closed");

    cluster.wait_for_buffered_request_count(0).await.expect(
        "the balancer must notice the half-closed client and release the buffered request \
         instead of holding it until buffered_request_timeout",
    );

    drop(client);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
