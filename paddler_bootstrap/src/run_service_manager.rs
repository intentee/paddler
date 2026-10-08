use anyhow::Error;
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
        .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use thiserror::Error;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;
    use trzcina::ServiceBundle;
    use trzcina::ServiceShutdownError;
    use trzcina::ServiceShutdownOptions;
    use trzcina::ServiceShutdownOutcome;
    use trzcina::ServiceShutdownOutcomeWithServiceName;

    use super::run_service_manager;

    #[derive(Debug, Error)]
    #[error("the bundle could not produce its services")]
    struct ServicesUnavailable;

    #[derive(Debug, Error)]
    #[error("the service could not run")]
    struct ServiceRunFailed;

    struct FailingServiceBundle;

    #[async_trait]
    impl ServiceBundle for FailingServiceBundle {
        async fn services(self) -> Result<Vec<Box<dyn Service>>> {
            Err(ServicesUnavailable.into())
        }
    }

    struct FailingService;

    #[async_trait]
    impl Service for FailingService {
        fn name(&self) -> &'static str {
            "failing_service"
        }

        async fn run(self: Box<Self>, _shutdown: CancellationToken) -> Result<()> {
            Err(ServiceRunFailed.into())
        }
    }

    struct BundleWithFailingService;

    #[async_trait]
    impl ServiceBundle for BundleWithFailingService {
        async fn services(self) -> Result<Vec<Box<dyn Service>>> {
            Ok(vec![Box::new(FailingService)])
        }
    }

    #[tokio::test]
    async fn propagates_bundle_registration_error() {
        let error = run_service_manager(
            FailingServiceBundle,
            CancellationToken::new(),
            ServiceShutdownOptions::default(),
        )
        .await
        .expect_err("a bundle without services must fail to start");

        assert!(error.downcast_ref::<ServicesUnavailable>().is_some());
    }

    #[tokio::test]
    async fn propagates_service_run_error() {
        let error = run_service_manager(
            BundleWithFailingService,
            CancellationToken::new(),
            ServiceShutdownOptions::default(),
        )
        .await
        .expect_err("a failing service must fail the service manager");
        let shutdown_error = error
            .downcast_ref::<ServiceShutdownError>()
            .expect("the service manager must report which services failed");

        assert!(matches!(
            shutdown_error.failed_outcomes(),
            [ServiceShutdownOutcomeWithServiceName {
                name: "failing_service",
                outcome: ServiceShutdownOutcome::Errored(service_error),
            }] if service_error.downcast_ref::<ServiceRunFailed>().is_some()
        ));
    }
}
