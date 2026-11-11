use reqwest::Client;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::HeaderValue;

use paddler_balancer::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;
use paddler_balancer::constants::OPENAI_HEADER_REQUEST_ID;
use paddler_messaging::constants::API_PATH_HEALTH;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn openai_service_lets_browsers_send_and_read_openai_headers() {
    let cluster = start_cluster(ClusterParams {
        inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a balancer without agents must start");

    let preflight = Client::new()
        .request(
            Method::OPTIONS,
            cluster
                .balancer
                .compat_openai_url(OPENAI_API_PATH_CHAT_COMPLETIONS)
                .expect("the cluster serves OpenAI"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            ["authorization", "content-type", OPENAI_HEADER_REQUEST_ID].join(", "),
        )
        .send()
        .await
        .expect("the preflight request must be answered");

    assert_eq!(preflight.status(), StatusCode::OK);

    let health = Client::new()
        .get(
            cluster
                .balancer
                .compat_openai_url(API_PATH_HEALTH)
                .expect("the cluster serves OpenAI"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header(OPENAI_HEADER_REQUEST_ID, "browser-request")
        .send()
        .await
        .expect("the health request must be answered");

    assert_eq!(
        health.headers().get("access-control-expose-headers"),
        Some(&HeaderValue::from_static(OPENAI_HEADER_REQUEST_ID))
    );
    assert_eq!(
        health.headers().get(OPENAI_HEADER_REQUEST_ID),
        Some(&HeaderValue::from_static("browser-request"))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
