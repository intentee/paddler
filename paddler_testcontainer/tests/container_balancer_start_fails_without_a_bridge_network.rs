#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_testcontainer::balancer_container::StartedBalancer;
use paddler_testcontainer::image_reference::ImageReference;

#[tokio::test(flavor = "multi_thread")]
async fn started_balancer_start_fails_when_the_container_has_no_bridge_network() -> Result<()> {
    let image = ImageReference::resolve()?;

    let result = StartedBalancer::start("none", &image, &BalancerServiceConfig::default()).await;

    assert!(result.is_err());

    Ok(())
}
