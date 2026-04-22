use actix_web::rt;
use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use log::error;
use log::info;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::service::Service;

#[derive(Default)]
pub struct ServiceManager {
    services: Vec<Box<dyn Service>>,
}

impl ServiceManager {
    pub fn add_service<TService: Service>(&mut self, service: TService) {
        self.services.push(Box::new(service));
    }

    pub async fn run_forever(self, shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
        let shutdown_token = CancellationToken::new();
        let mut service_handles = FuturesUnordered::new();

        for mut service in self.services {
            let service_name = service.name().to_owned();
            let service_token = shutdown_token.clone();

            service_handles.push(rt::spawn(async move {
                info!("{service_name}: Starting");

                match service.run(service_token).await {
                    Ok(()) => info!("{service_name}: Stopped"),
                    Err(err) => error!("{service_name}: {err}"),
                }
            }));
        }

        tokio::select! {
            _ = shutdown_rx => {}
            _ = service_handles.next() => {}
        }

        shutdown_token.cancel();

        while service_handles.next().await.is_some() {}

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::anyhow;
    use async_trait::async_trait;
    use tokio::sync::Notify;

    use super::*;

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

            Err(anyhow!("boom"))
        }
    }

    struct ImmediatelyFailingService;

    #[async_trait]
    impl Service for ImmediatelyFailingService {
        fn name(&self) -> &'static str {
            "test::immediately_failing_service"
        }

        async fn run(&mut self, _shutdown: CancellationToken) -> Result<()> {
            Err(anyhow!("boom"))
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
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });
        manager.add_service(FailingOnDemandService { fail: fail.clone() });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown_rx));

        ready.notified().await;
        fail.notify_one();

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn ok_exit_cascades_to_peers() -> Result<()> {
        let ready = Arc::new(Notify::new());
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });
        manager.add_service(ImmediatelySuccessService);

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown_rx));

        ready.notified().await;

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn fast_failure_cascades_to_late_subscriber() -> Result<()> {
        let ready = Arc::new(Notify::new());
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut manager = ServiceManager::default();
        manager.add_service(ImmediatelyFailingService);
        manager.add_service(NeverExitingService {
            ready: ready.clone(),
        });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown_rx));

        ready.notified().await;

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn all_services_exit_before_cancel_is_idempotent() -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut manager = ServiceManager::default();
        manager.add_service(ImmediatelySuccessService);
        manager.add_service(ImmediatelySuccessService);
        manager.add_service(ImmediatelySuccessService);

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown_rx));

        manager_handle.await??;

        Ok(())
    }

    #[actix_web::test]
    async fn external_shutdown_still_works() -> Result<()> {
        let ready_first = Arc::new(Notify::new());
        let ready_second = Arc::new(Notify::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut manager = ServiceManager::default();
        manager.add_service(NeverExitingService {
            ready: ready_first.clone(),
        });
        manager.add_service(NeverExitingService {
            ready: ready_second.clone(),
        });

        let manager_handle = actix_web::rt::spawn(manager.run_forever(shutdown_rx));

        ready_first.notified().await;
        ready_second.notified().await;

        if let Err(_unsent_signal) = shutdown_tx.send(()) {
            return Err(anyhow!("run_forever dropped its shutdown receiver"));
        }

        manager_handle.await??;

        Ok(())
    }
}
