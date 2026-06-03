use anyhow::Result;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceBundle;
use trzcina::ServiceManager;
use trzcina::ServiceShutdownOptions;

pub async fn run_service_manager<TServiceBundle: ServiceBundle>(
    bundle: TServiceBundle,
    task_shutdown: CancellationToken,
    shutdown_options: ServiceShutdownOptions,
) -> Result<()> {
    let mut service_manager = ServiceManager::default();

    service_manager.register_bundle(bundle).await?;
    service_manager
        .start(task_shutdown)
        .run_to_completion(shutdown_options)
        .await
        .into_result()
        .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::anyhow;
    use async_trait::async_trait;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;
    use trzcina::ServiceBundle;
    use trzcina::ServiceShutdownOptions;

    use super::run_service_manager;

    struct FailingServiceBundle;

    #[async_trait]
    impl ServiceBundle for FailingServiceBundle {
        async fn services(self) -> Result<Vec<Box<dyn Service>>> {
            Err(anyhow!("service bundle failed to produce services"))
        }
    }

    #[tokio::test]
    async fn propagates_bundle_registration_error() {
        let result = run_service_manager(
            FailingServiceBundle,
            CancellationToken::new(),
            ServiceShutdownOptions::default(),
        )
        .await;

        assert!(result.is_err());
    }
}
