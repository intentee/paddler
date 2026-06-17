#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_testcontainer::balancer_container::StartedBalancer;
use paddler_testcontainer::image_reference::ImageReference;

#[tokio::test(flavor = "multi_thread")]
async fn started_balancer_start_fails_for_an_invalid_image_reference() -> Result<()> {
    let invalid_image = ImageReference {
        name: "Invalid_Uppercase_Repository_Name".to_owned(),
        tag: "does-not-exist".to_owned(),
    };

    let result =
        StartedBalancer::start("paddler-test", &invalid_image, &BalancerServiceConfig::default())
            .await;

    assert!(result.is_err());

    Ok(())
}
