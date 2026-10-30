use reqwest::Url;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error;
use paddler_client::format_api_url::format_api_url;
use paddler_client::http_client::HttpClient;
use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn http_client_reports_a_body_that_is_not_json_as_undecodable() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");
    let health_url = format_api_url(&cluster.balancer.management_base_url, ApiPath::HEALTH);

    let health_as_json = HttpClient::new(cluster.balancer.management_base_url.clone())
        .get_json::<String>(CancellationToken::new(), ApiPath::HEALTH)
        .await;

    assert!(matches!(
        health_as_json,
        Err(Error::Http(decoding_error))
            if decoding_error.is_decode()
                && decoding_error.url().map(Url::as_str) == Some(health_url.as_str())
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
