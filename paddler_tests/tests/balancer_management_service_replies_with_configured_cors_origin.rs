use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::send_cors_preflight::send_cors_preflight;
use paddler_tests::start_cluster::start_cluster;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_management_service_replies_with_configured_cors_origin() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        management_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let health_url = cluster
        .balancer
        .management_base_url()
        .expect("the management service must have a base URL")
        .join("health")
        .expect("the health path must join onto the base URL");

    let response = send_cors_preflight(health_url, ALLOWED_ORIGIN)
        .await
        .expect("the preflight request must succeed");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .expect("the preflight response must allow an origin")
            .to_str()
            .expect("the allowed origin must be ASCII"),
        ALLOWED_ORIGIN
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
