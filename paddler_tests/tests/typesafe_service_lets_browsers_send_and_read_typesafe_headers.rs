use reqwest::Client;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::HeaderValue;

use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::decision_cluster_without_agents_params::decision_cluster_without_agents_params;
use paddler_tests::start_cluster::start_cluster;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_service_lets_browsers_send_and_read_typesafe_headers() {
    let cluster = start_cluster(ClusterParams {
        inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        ..decision_cluster_without_agents_params()
    })
    .await
    .expect("a decision balancer without agents must start");
    let typesafe_base_url = cluster
        .balancer
        .compat_typesafe_base_url()
        .expect("the cluster serves TypeSafe");

    let preflight = Client::new()
        .request(
            Method::OPTIONS,
            typesafe_base_url
                .join(TypeSafeApiPath::SYSTEM_ONE)
                .expect("the path must join"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            [
                "authorization",
                "content-type",
                TypeSafeHeader::REQUEST_ID,
                TypeSafeHeader::RETRY_COUNT,
                TypeSafeHeader::RUNTIME,
                TypeSafeHeader::SDK,
            ]
            .join(", "),
        )
        .send()
        .await
        .expect("the preflight request must be answered");

    assert_eq!(preflight.status(), StatusCode::OK);

    let models = Client::new()
        .get(
            typesafe_base_url
                .join(TypeSafeApiPath::MODELS)
                .expect("the path must join"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .send()
        .await
        .expect("the models request must be answered");

    assert_eq!(
        models.headers().get("access-control-expose-headers"),
        Some(&HeaderValue::from_static(TypeSafeHeader::REQUEST_ID))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
