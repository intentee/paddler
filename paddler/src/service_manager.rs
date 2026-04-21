use actix_web::rt;
use anyhow::Result;
use futures::future::join_all;
use log::error;
use log::info;
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
        let mut service_handles = Vec::with_capacity(self.services.len());

        for mut service in self.services {
            let service_name = service.name().to_owned();
            let service_shutdown_rx = shutdown_broadcast_tx.subscribe();

            service_handles.push(rt::spawn(async move {
                info!("{service_name}: Starting");

                match service.run(service_shutdown_rx).await {
                    Ok(()) => info!("{service_name}: Stopped"),
                    Err(err) => error!("{service_name}: {err}"),
                }
            }));
        }

        shutdown_rx.await?;
        shutdown_broadcast_tx.send(())?;
        join_all(service_handles).await;

        Ok(())
    }
}
