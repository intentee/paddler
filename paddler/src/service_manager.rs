use actix_web::rt;
use actix_web::rt::task::JoinError;
use anyhow::Result;
use anyhow::anyhow;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use log::error;
use log::info;
use tokio_util::sync::CancellationToken;

use crate::service::Service;

fn extract_service_error(join_result: Result<Result<()>, JoinError>) -> Option<anyhow::Error> {
    match join_result {
        Ok(Ok(())) => None,
        Ok(Err(service_error)) => Some(service_error),
        Err(join_error) => Some(anyhow!("service task panicked: {join_error}")),
    }
}

#[derive(Default)]
pub struct ServiceManager {
    services: Vec<Box<dyn Service>>,
}

impl ServiceManager {
    pub fn add_service<TService: Service>(&mut self, service: TService) {
        self.services.push(Box::new(service));
    }

    pub async fn run_forever(self, shutdown: CancellationToken) -> Result<()> {
        let service_token = shutdown.child_token();
        let mut service_handles = FuturesUnordered::new();

        for mut service in self.services {
            let service_name = service.name().to_owned();
            let task_token = service_token.clone();

            service_handles.push(rt::spawn(async move {
                info!("{service_name}: Starting");

                let result = service.run(task_token).await;

                match &result {
                    Ok(()) => info!("{service_name}: Stopped"),
                    Err(service_error) => error!("{service_name}: {service_error}"),
                }

                result
            }));
        }

        let mut first_error: Option<anyhow::Error> = None;

        tokio::select! {
            () = shutdown.cancelled() => {}
            Some(join_result) = service_handles.next() => {
                first_error = extract_service_error(join_result);
            }
        }

        service_token.cancel();

        while let Some(join_result) = service_handles.next().await {
            if let Some(service_error) = extract_service_error(join_result)
                && first_error.is_none()
            {
                first_error = Some(service_error);
            }
        }

        first_error.map_or_else(|| Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use thiserror::Error;
    use tokio::sync::Notify;

    use super::*;

    #[derive(Debug, Error)]
    #[error("intentional test failure")]
    struct TestFailureMarker;

    struct NeverExitingService {
        ready: Arc<Notify>,
    }

    #[async_trait]
    impl Service for NeverExitingService {
        fn name(&self) -> &'static str {
            "test::never_exiting_service"
        }

        async fn run(&mut self, shutdown: CancellationToken) -> Result<()> {
            self.ready.notify_one();

            shutdown.cancelled().await;

            Ok(())
        }
    }

    struct FailingOnDemandService {
        fail: Arc<Notify>,
    }

    #[async_trait]
    impl Service for FailingOnDemandService {
        fn name(&self) -> &'static str {
            "test::failing_on_demand_service"
        }

        async fn run(&mut self, _shutdown: CancellationToken) -> Result<()> {
            self.fail.notified().await;

            Err(TestFailureMarker.into())
        }
    }

    struct ImmediatelyFailingService;

    #[async_trait]
    impl Service for ImmediatelyFailingService {
        fn name(&self) -> &'static str {
            "test::immediately_failing_service"
        }

        async fn run(&mut self, _shutdown: CancellationToken) -> Result<()> {
            Err(TestFailureMarker.into())
        }
    }

    struct ImmediatelySuccessService;

    #[async_trait]
    impl Service for ImmediatelySuccessService {
        fn name(&self) -> &'static str {
            "test::immediately_success_service"
        }

        async fn run(&mut self, _shutdown: CancellationToken) -> Result<()> {
            Ok(())
        }
    }

    #[actix_web::test]
    async fn err_exit_cascades_to_peers() -> Result<()> {
        let ready = Arc::new(Notify::new());
        let fail = Arc::new(Notify::new());
        let shutdown = CancellationToken::new();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });
        manager.add_service(FailingOnDemandService { fail: fail.clone() });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown));

        ready.notified().await;
        fail.notify_one();

        let error = match manager_handle.await? {
            Ok(()) => {
                return Err(anyhow!(
                    "run_forever should surface the failing service's error"
                ));
            }
            Err(service_error) => service_error,
        };

        error
            .downcast_ref::<TestFailureMarker>()
            .ok_or_else(|| anyhow!("expected TestFailureMarker, got: {error:?}"))?;

        Ok(())
    }

    #[actix_web::test]
    async fn ok_exit_cascades_to_peers() -> Result<()> {
        let ready = Arc::new(Notify::new());
        let shutdown = CancellationToken::new();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });
        manager.add_service(ImmediatelySuccessService);

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown));

        ready.notified().await;

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn fast_failure_cascades_to_late_subscriber() -> Result<()> {
        let ready = Arc::new(Notify::new());
        let shutdown = CancellationToken::new();

        let mut manager = ServiceManager::default();
        manager.add_service(ImmediatelyFailingService);
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown));

        ready.notified().await;

        let error = match manager_handle.await? {
            Ok(()) => {
                return Err(anyhow!(
                    "run_forever should surface the failing service's error"
                ));
            }
            Err(service_error) => service_error,
        };

        error
            .downcast_ref::<TestFailureMarker>()
            .ok_or_else(|| anyhow!("expected TestFailureMarker, got: {error:?}"))?;

        Ok(())
    }

    #[actix_web::test]
    async fn all_services_exit_before_cancel_is_idempotent() -> Result<()> {
        let shutdown = CancellationToken::new();

        let mut manager = ServiceManager::default();
        manager.add_service(ImmediatelySuccessService);
        manager.add_service(ImmediatelySuccessService);
        manager.add_service(ImmediatelySuccessService);

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown));

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn external_shutdown_still_works() -> Result<()> {
        let ready_first = Arc::new(Notify::new());
        let ready_second = Arc::new(Notify::new());
        let shutdown = CancellationToken::new();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready_first.clone(),
        });
        manager.add_service(NeverExitingService {
            ready: ready_second.clone(),
        });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown.clone()));

        ready_first.notified().await;
        ready_second.notified().await;

        shutdown.cancel();

        manager_handle.await??;

        Ok(())
    }
}
