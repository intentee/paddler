#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_testcontainer::balancer_container::StartedBalancer;
use paddler_testcontainer::image_reference::ImageReference;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;

#[tokio::test(flavor = "multi_thread")]
async fn started_balancer_from_container_fails_without_published_ports() -> Result<()> {
    let image = ImageReference::resolve()?;

    let container = GenericImage::new(image.name, image.tag)
        .with_cmd(["balancer".to_owned()])
        .start()
        .await?;

    let result = StartedBalancer::from_container(container).await;

    assert!(result.is_err());

    Ok(())
}
