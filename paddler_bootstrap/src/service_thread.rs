use std::future::Future;
use std::thread::JoinHandle;

use anyhow::Result;
use anyhow::anyhow;
use log::error;
use log::warn;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub struct ServiceThread {
    cancellation_token: CancellationToken,
    completion_rx: Option<oneshot::Receiver<Result<()>>>,
    thread: Option<JoinHandle<()>>,
}

impl ServiceThread {
    pub fn spawn<TRun, TFuture>(cancellation_token: CancellationToken, run: TRun) -> Self
    where
        TRun: FnOnce(CancellationToken) -> TFuture + Send + 'static,
        TFuture: Future<Output = Result<()>>,
    {
        let task_token = cancellation_token.clone();
        let (completion_tx, completion_rx) = oneshot::channel::<Result<()>>();

        let thread = std::thread::spawn(move || {
            let result = actix_web::rt::System::new().block_on(run(task_token));
            if let Err(unsent) = completion_tx.send(result) {
                match unsent {
                    Ok(()) => warn!(
                        "service thread completion receiver dropped before delivery; run() succeeded but result was not observed by the caller"
                    ),
                    Err(run_err) => error!(
                        "service thread completion receiver dropped before delivery; lost run() error: {run_err:?}"
                    ),
                }
            }
        });

        Self {
            cancellation_token,
            completion_rx: Some(completion_rx),
            thread: Some(thread),
        }
    }

    pub fn wait_for_completion(&mut self) -> impl Future<Output = Result<()>> + Send + 'static {
        let completion_rx = self.completion_rx.take();

        async move {
            let completion_rx = completion_rx
                .ok_or_else(|| anyhow!("service thread completion already consumed"))?;

            completion_rx.await.map_err(|error| {
                anyhow!("service thread dropped before reporting completion: {error}")
            })?
        }
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }
}

impl Drop for ServiceThread {
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        if let Some(thread) = self.thread.take()
            && let Err(panic_payload) = thread.join()
        {
            error!("service thread panicked: {panic_payload:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn service_thread_finishes_cleanly_when_completion_receiver_dropped_after_success() {
        let cancellation_token = CancellationToken::new();
        let mut service_thread = ServiceThread::spawn(
            cancellation_token.clone(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;

                Ok(())
            },
        );

        drop(service_thread.wait_for_completion());

        cancellation_token.cancel();

        drop(service_thread);
    }

    #[tokio::test]
    async fn service_thread_finishes_cleanly_when_completion_receiver_dropped_after_failure() {
        let cancellation_token = CancellationToken::new();
        let mut service_thread = ServiceThread::spawn(
            cancellation_token.clone(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;

                Err(anyhow!("service run failed"))
            },
        );

        drop(service_thread.wait_for_completion());

        cancellation_token.cancel();

        drop(service_thread);
    }

    #[tokio::test]
    async fn wait_for_completion_errors_when_service_thread_panics() {
        let mut service_thread =
            ServiceThread::spawn(CancellationToken::new(), |_task_cancellation_token| async {
                panic!("service thread crashed")
            });

        let completion_result = service_thread.wait_for_completion().await;

        assert!(completion_result.is_err());
    }

    #[tokio::test]
    async fn wait_for_completion_errors_when_called_twice() {
        let cancellation_token = CancellationToken::new();
        let mut service_thread = ServiceThread::spawn(
            cancellation_token.clone(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;

                Ok(())
            },
        );

        cancellation_token.cancel();

        let first_completion = service_thread.wait_for_completion().await;
        let second_completion = service_thread.wait_for_completion().await;

        assert!(first_completion.is_ok());
        assert!(second_completion.is_err());
    }

    #[tokio::test]
    async fn cancel_stops_the_running_service_thread() {
        let mut service_thread = ServiceThread::spawn(
            CancellationToken::new(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;

                Ok(())
            },
        );

        service_thread.cancel();

        let completion_result = service_thread.wait_for_completion().await;

        assert!(completion_result.is_ok());
    }
}
