use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_management_service_replies_with_configured_cors_origin() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        management_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let http_client = reqwest::Client::new();
    let management_health_url = cluster
        .balancer
        .addresses
        .management_base_url()?
        .join("health")?;

    let response = http_client
        .request(reqwest::Method::OPTIONS, management_health_url)
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
