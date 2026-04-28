#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_management_service_replies_with_configured_cors_origin() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 0,
        management_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let http_client = reqwest::Client::new();
    let management_health_url = cluster.addresses.management_base_url()?.join("health")?;

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
