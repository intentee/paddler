use http::Response as HttpResponse;
use reqwest::Response;
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
    let health_body_decoding_error = Response::from(HttpResponse::new("OK"))
        .json::<String>()
        .await
        .expect_err("the health body must not decode as a JSON string");

    assert_eq!(
        http_client
            .get_json::<String>(CancellationToken::new(), ApiPath::HEALTH)
            .await
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(Error::Http(health_body_decoding_error).to_string())
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
