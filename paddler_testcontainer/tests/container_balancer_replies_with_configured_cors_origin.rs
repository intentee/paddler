#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_testcontainer::container_cluster_backend::ContainerClusterBackend;

const ALLOWED_ORIGIN: &str = "http://example.com";

#[tokio::test(flavor = "multi_thread")]
async fn container_balancer_replies_with_configured_cors_origin() -> Result<()> {
    let cluster = Cluster::start(
        &ContainerClusterBackend::default().with_service_config(BalancerServiceConfig {
            inference_cors_allowed_hosts: vec![ALLOWED_ORIGIN.to_owned()],
            ..BalancerServiceConfig::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            desired_state: DesiredStateInit::Inherit,
            wait_for_slots_ready: false,
        },
    )
    .await?;

    let preflight = cluster
        .inference_client
        .cors_preflight(ALLOWED_ORIGIN)
        .await?;

    assert_eq!(preflight.status, 200);
    assert_eq!(preflight.allow_origin, ALLOWED_ORIGIN);

    cluster.shutdown().await?;

    Ok(())
}
