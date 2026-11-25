use serde_json::json;
use tokio::join;

use paddler_client::error::Error;
use paddler_client::inference_socket::connection::Connection;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_connection_rejects_requests_after_the_balancer_closes_it() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let connection = Connection::connect(cluster.balancer.inference_base_url.clone())
        .await
        .expect("the connection to the balancer must open");

    join!(connection.disconnected(), async {
        cluster
            .shutdown()
            .await
            .expect("the cluster must shut down cleanly");
    });

    assert!(matches!(
        connection.send("request-after-close".to_owned(), &json!({})),
        Err(Error::ConnectionDropped { request_id }) if request_id == "request-after-close"
    ));
}
