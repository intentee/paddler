use reqwest::Client;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::HeaderValue;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_balancer::compatibility::openai_service::openai_header::OpenAIHeader;
use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn openai_service_lets_browsers_send_and_read_openai_headers() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer without agents must start");
    let openai_base_url = cluster
        .balancer
        .compat_openai_base_url()
        .expect("the cluster serves OpenAI");

    let preflight = Client::new()
        .request(
            Method::OPTIONS,
            openai_base_url
                .join(OpenAIApiPath::CHAT_COMPLETIONS)
                .expect("the path must join"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            ["authorization", "content-type", OpenAIHeader::REQUEST_ID].join(", "),
        )
        .send()
        .await
        .expect("the preflight request must be answered");

    assert_eq!(preflight.status(), StatusCode::OK);

    let health = Client::new()
        .get(
            openai_base_url
                .join(ApiPath::HEALTH)
                .expect("the path must join"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header(OpenAIHeader::REQUEST_ID, "browser-request")
        .send()
        .await
        .expect("the health request must be answered");

    assert_eq!(
        health.headers().get("access-control-expose-headers"),
        Some(&HeaderValue::from_static(OpenAIHeader::REQUEST_ID))
    );
    assert_eq!(
        health.headers().get(OpenAIHeader::REQUEST_ID),
        Some(&HeaderValue::from_static("browser-request"))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
