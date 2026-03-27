use anyhow::Result;
use tokio::sync::broadcast;

pub async fn run_until_shutdown<TService>(
    mut shutdown: broadcast::Receiver<()>,
    mut service: TService,
) -> Result<()>
where
    TService: AsyncFnMut(broadcast::Receiver<()>) -> Result<()>,
{
    loop {
        let resubscribed_shutdown = shutdown.resubscribe();

        tokio::select! {
            biased;
            _ = shutdown.recv() => return Ok(()),
            result = service(resubscribed_shutdown) => result?,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;

    use anyhow::Result;
    use tokio::sync::broadcast;

    use super::run_until_shutdown;

    #[actix_web::test]
    async fn stops_when_shutdown_sent_during_service() -> Result<()> {
        let (shutdown_tx, initial_rx) = broadcast::channel::<()>(1);
        drop(initial_rx);
        let shutdown_rx = shutdown_tx.subscribe();

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        let shutdown_tx_clone = shutdown_tx.clone();

        run_until_shutdown(
            shutdown_rx,
            async |mut inner_shutdown: broadcast::Receiver<()>| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);

                shutdown_tx_clone.send(()).ok();
                inner_shutdown.recv().await.ok();

                Ok(())
            },
        )
        .await?;

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Service ran more than once after shutdown was sent"
        );

        Ok(())
    }

    #[actix_web::test]
    async fn does_not_run_service_when_shutdown_already_sent() -> Result<()> {
        let (shutdown_tx, initial_rx) = broadcast::channel::<()>(1);
        drop(initial_rx);
        let shutdown_rx = shutdown_tx.subscribe();

        shutdown_tx.send(())?;

        let service_ran = Arc::new(AtomicBool::new(false));
        let service_ran_clone = service_ran.clone();

        run_until_shutdown(
            shutdown_rx,
            async |_inner_shutdown: broadcast::Receiver<()>| {
                service_ran_clone.store(true, Ordering::SeqCst);

                Ok(())
            },
        )
        .await?;

        assert!(
            !service_ran.load(Ordering::SeqCst),
            "Service should not run when shutdown was already sent"
        );

        Ok(())
    }
}
