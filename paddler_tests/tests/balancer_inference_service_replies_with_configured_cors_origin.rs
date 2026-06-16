use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_service_replies_with_configured_cors_origin() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::new(BalancerServiceConfig {
            inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
            ..BalancerServiceConfig::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    let http_client = reqwest::Client::new();
    let inference_health_url = cluster
        .balancer
        .addresses
        .inference_base_url()?
        .join("health")?;

    let response = http_client
        .request(reqwest::Method::OPTIONS, inference_health_url)
        .header("Origin", ALLOWED_ORIGIN)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .context("preflight request should succeed")?;

    assert_eq!(response.status(), 200);

    let cors_origin = response
        .headers()
        .get("access-control-allow-origin")
        .context("missing Access-Control-Allow-Origin header")?
        .to_str()
        .context("CORS header should be valid ASCII")?;

    assert_eq!(cors_origin, ALLOWED_ORIGIN);

    cluster.shutdown().await?;

    Ok(())
}
