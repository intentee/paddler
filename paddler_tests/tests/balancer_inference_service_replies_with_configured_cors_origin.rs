use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_service_replies_with_configured_cors_origin() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default().with_service_config(BalancerServiceConfig {
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

    let preflight = cluster.inference_client.cors_preflight(ALLOWED_ORIGIN).await?;

    assert_eq!(preflight.status, 200);
    assert_eq!(preflight.allow_origin, ALLOWED_ORIGIN);

    cluster.shutdown().await?;

    Ok(())
}
