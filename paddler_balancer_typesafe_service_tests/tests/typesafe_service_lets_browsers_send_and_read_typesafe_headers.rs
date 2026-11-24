use reqwest::Client;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::HeaderValue;

use paddler_balancer_typesafe_service::constants::TYPESAFE_API_PATH_MODELS;
use paddler_balancer_typesafe_service::constants::TYPESAFE_API_PATH_SYSTEM_ONE;
use paddler_balancer_typesafe_service::constants::TYPESAFE_HEADER_REQUEST_ID;
use paddler_balancer_typesafe_service::constants::TYPESAFE_HEADER_RETRY_COUNT;
use paddler_balancer_typesafe_service::constants::TYPESAFE_HEADER_RUNTIME;
use paddler_balancer_typesafe_service::constants::TYPESAFE_HEADER_SDK;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_service_lets_browsers_send_and_read_typesafe_headers() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams {
            inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
            ..ClusterParams::default()
        })
        .await
        .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;

    let preflight = Client::new()
        .request(
            Method::OPTIONS,
            cluster
                .balancer
                .compat_typesafe_url(TYPESAFE_API_PATH_SYSTEM_ONE)
                .expect("the cluster serves TypeSafe"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            [
                "authorization",
                "content-type",
                TYPESAFE_HEADER_REQUEST_ID,
                TYPESAFE_HEADER_RETRY_COUNT,
                TYPESAFE_HEADER_RUNTIME,
                TYPESAFE_HEADER_SDK,
            ]
            .join(", "),
        )
        .send()
        .await
        .expect("the preflight request must be answered");

    assert_eq!(preflight.status(), StatusCode::OK);

    let models = Client::new()
        .get(
            cluster
                .balancer
                .compat_typesafe_url(TYPESAFE_API_PATH_MODELS)
                .expect("the cluster serves TypeSafe"),
        )
        .header("Origin", ALLOWED_ORIGIN)
        .send()
        .await
        .expect("the models request must be answered");

    assert_eq!(
        models.headers().get("access-control-expose-headers"),
        Some(&HeaderValue::from_static(TYPESAFE_HEADER_REQUEST_ID))
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
