use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error;
use paddler_client::http_client::HttpClient;
use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn http_client_reports_a_server_sent_event_of_another_shape_as_undecodable() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer without agents must start");
    let mut events = HttpClient::new(
        cluster
            .balancer
            .management_base_url()
            .expect("the balancer must expose its management address"),
    )
    .get_sse_json::<String>(CancellationToken::new(), ApiPath::AGENTS_STREAM)
    .await
    .expect("the agents event stream must open");

    assert!(matches!(
        events.next().await,
        Some(Err(Error::Json(source))) if source.is_data()
    ));

    drop(events);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
