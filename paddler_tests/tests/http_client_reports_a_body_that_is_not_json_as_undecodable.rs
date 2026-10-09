use tokio_util::sync::CancellationToken;

use paddler_client::error::Error;
use paddler_client::http_client::HttpClient;
use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn http_client_reports_a_body_that_is_not_json_as_undecodable() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer without agents must start");
    let http_client = HttpClient::new(
        cluster
            .balancer
            .management_base_url()
            .expect("the balancer must expose its management address"),
    );

    assert!(matches!(
        http_client
            .get_json::<String>(CancellationToken::new(), ApiPath::HEALTH)
            .await,
        Err(Error::Http(source)) if source.is_decode()
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
