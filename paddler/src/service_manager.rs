use std::sync::Arc;

use actix_web::rt;
use anyhow::Result;
use futures::future::join_all;
use log::error;
use log::info;
use log::warn;
use tokio::sync::broadcast;
use tokio::sync::oneshot;

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
        let (shutdown_broadcast_tx, _) = broadcast::channel::<()>(1);
        let shutdown_broadcast_tx_arc = Arc::new(shutdown_broadcast_tx.clone());
        let mut service_handles = Vec::with_capacity(self.services.len());

        for mut service in self.services {
            let service_name = service.name().to_owned();
            let shutdown_broadcast_tx_arc_clone = shutdown_broadcast_tx_arc.clone();
            let service_shutdown_rx = shutdown_broadcast_tx_arc.subscribe();

            service_handles.push(rt::spawn(async move {
                info!("{service_name}: Starting");

                match service.run(service_shutdown_rx).await {
                    Ok(()) => info!("{service_name}: Stopped"),
                    Err(err) => {
                        error!("{service_name}: {err}");

                        if let Err(err) = shutdown_broadcast_tx_arc_clone.send(()) {
                            warn!("{service_name}: Failed to send shutdown signal: {err}");
                        }
                    }
                }
            }));
        }

        shutdown_rx.await?;

        if let Err(err) = shutdown_broadcast_tx.send(()) {
            error!("Failed to send shutdown signal to services: {err:#?}");
        }

        join_all(service_handles).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use anyhow::Result;
    use async_trait::async_trait;
    use tokio::sync::Notify;
    use tokio::sync::broadcast;
    use tokio::sync::oneshot;

    use super::ServiceManager;
    use crate::service::Service;

    struct SlowShutdownService {
        cleanup_completed: Arc<AtomicBool>,
        ready_notify: Arc<Notify>,
    }

    #[async_trait]
    impl Service for SlowShutdownService {
        fn name(&self) -> &'static str {
            "test::slow_shutdown_service"
        }

        async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
            self.ready_notify.notify_one();
            shutdown_rx.recv().await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            self.cleanup_completed.store(true, Ordering::Release);

            Ok(())
        }
    }

    #[actix_web::test]
    async fn services_complete_graceful_shutdown() -> Result<()> {
        let cleanup_completed = Arc::new(AtomicBool::new(false));
        let ready_notify = Arc::new(Notify::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut service_manager = ServiceManager::default();
        service_manager.add_service(SlowShutdownService {
            cleanup_completed: cleanup_completed.clone(),
            ready_notify: ready_notify.clone(),
        });

        let manager_handle =
            actix_web::rt::spawn(async move { service_manager.run_forever(shutdown_rx).await });

        ready_notify.notified().await;

        shutdown_tx
            .send(())
            .map_err(|()| anyhow::anyhow!("Failed to send shutdown signal"))?;

        manager_handle.await??;

        assert!(
            cleanup_completed.load(Ordering::Acquire),
            "Service cleanup did not complete before run_forever returned"
        );

        Ok(())
    }

    struct FailingService {
        fail_notify: Arc<Notify>,
    }

    #[async_trait]
    impl Service for FailingService {
        fn name(&self) -> &'static str {
            "test::failing_service"
        }

        async fn run(&mut self, shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
            drop(shutdown_rx);
            self.fail_notify.notified().await;

            Err(anyhow::anyhow!("service failure"))
        }
    }

    struct CascadeListenerService {
        cascade_received: Arc<AtomicBool>,
        cascade_completed_notify: Arc<Notify>,
        ready_notify: Arc<Notify>,
    }

    #[async_trait]
    impl Service for CascadeListenerService {
        fn name(&self) -> &'static str {
            "test::cascade_listener_service"
        }

        async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
            self.ready_notify.notify_one();
            shutdown_rx.recv().await?;
            self.cascade_received.store(true, Ordering::Release);
            self.cascade_completed_notify.notify_one();

            Ok(())
        }
    }

    #[actix_web::test]
    async fn service_failure_cascades_shutdown_to_others() -> Result<()> {
        let cascade_received = Arc::new(AtomicBool::new(false));
        let cascade_completed_notify = Arc::new(Notify::new());
        let listener_ready_notify = Arc::new(Notify::new());
        let fail_notify = Arc::new(Notify::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut service_manager = ServiceManager::default();

        service_manager.add_service(FailingService {
            fail_notify: fail_notify.clone(),
        });

        service_manager.add_service(CascadeListenerService {
            cascade_received: cascade_received.clone(),
            cascade_completed_notify: cascade_completed_notify.clone(),
            ready_notify: listener_ready_notify.clone(),
        });

        let manager_handle =
            actix_web::rt::spawn(async move { service_manager.run_forever(shutdown_rx).await });

        listener_ready_notify.notified().await;
        fail_notify.notify_one();
        cascade_completed_notify.notified().await;

        shutdown_tx
            .send(())
            .map_err(|()| anyhow::anyhow!("Failed to send shutdown signal"))?;

        manager_handle.await??;

        assert!(
            cascade_received.load(Ordering::Acquire),
            "Cascade shutdown did not reach the listener service"
        );

        Ok(())
    }

    struct ImmediatelyFailingService;

    #[async_trait]
    impl Service for ImmediatelyFailingService {
        fn name(&self) -> &'static str {
            "test::immediately_failing_service"
        }

        async fn run(&mut self, shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
            drop(shutdown_rx);

            Err(anyhow::anyhow!("immediate failure"))
        }
    }

    #[actix_web::test]
    async fn fast_failure_cascades_to_late_subscribers() -> Result<()> {
        let cascade_received = Arc::new(AtomicBool::new(false));
        let cascade_completed_notify = Arc::new(Notify::new());
        let listener_ready_notify = Arc::new(Notify::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let mut service_manager = ServiceManager::default();

        service_manager.add_service(ImmediatelyFailingService);

        service_manager.add_service(CascadeListenerService {
            cascade_received: cascade_received.clone(),
            cascade_completed_notify: cascade_completed_notify.clone(),
            ready_notify: listener_ready_notify.clone(),
        });

        let manager_handle =
            actix_web::rt::spawn(async move { service_manager.run_forever(shutdown_rx).await });

        let cascade_result =
            tokio::time::timeout(Duration::from_secs(1), cascade_completed_notify.notified()).await;

        shutdown_tx
            .send(())
            .map_err(|()| anyhow::anyhow!("Failed to send shutdown signal"))?;

        manager_handle.await??;

        assert!(
            cascade_result.is_ok(),
            "Cascade shutdown did not reach the late-subscribing service"
        );

        Ok(())
    }
}
