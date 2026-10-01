use std::convert::Infallible;
use std::future::Future;
use std::thread::JoinHandle;
use std::thread::spawn;

use actix_web::rt::System;
use anyhow::Result;
use log::error;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::bootstrap_error::BootstrapError;

pub struct ServiceThread {
    cancellation_token: CancellationToken,
    finished_rx: oneshot::Receiver<Infallible>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl ServiceThread {
    pub fn spawn<TRun, TFuture>(cancellation_token: CancellationToken, run: TRun) -> Self
    where
        TRun: FnOnce(CancellationToken) -> TFuture + Send + 'static,
        TFuture: Future<Output = Result<()>>,
    {
        let task_token = cancellation_token.clone();
        let (finished_tx, finished_rx) = oneshot::channel::<Infallible>();

        let thread = spawn(move || {
            let _finished_tx = finished_tx;

            System::new().block_on(run(task_token))
        });

        Self {
            cancellation_token,
            finished_rx,
            thread: Some(thread),
        }
    }

    pub async fn wait_for_completion(mut self) -> Result<(), BootstrapError> {
        let Err(_thread_finished) = (&mut self.finished_rx).await;

        self.join_thread()
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    fn join_thread(&mut self) -> Result<(), BootstrapError> {
        self.thread
            .take()
            .map_or(Ok(()), |thread| match thread.join() {
                Ok(run_result) => {
                    run_result.map_err(|source| BootstrapError::ServiceRunFailed { source })
                }
                Err(_panic_payload) => Err(BootstrapError::ServiceThreadPanicked),
            })
    }
}

impl Drop for ServiceThread {
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        if let Err(run_error) = self.join_thread() {
            error!("service thread stopped with an error nobody waited for: {run_error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    use anyhow::Error;
    use tokio_util::sync::CancellationToken;

    use super::ServiceThread;
    use crate::bootstrap_error::BootstrapError;

    #[tokio::test]
    async fn dropping_a_service_thread_stops_and_joins_it() {
        let run_finished = Arc::new(AtomicBool::new(false));
        let run_finished_flag = run_finished.clone();
        let service_thread = ServiceThread::spawn(
            CancellationToken::new(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;
                run_finished_flag.store(true, Ordering::Release);

                Ok(())
            },
        );

        drop(service_thread);

        assert!(run_finished.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn dropping_a_service_thread_joins_it_after_its_run_failed() {
        let run_finished = Arc::new(AtomicBool::new(false));
        let run_finished_flag = run_finished.clone();
        let service_thread = ServiceThread::spawn(
            CancellationToken::new(),
            |task_cancellation_token| async move {
                task_cancellation_token.cancelled().await;
                run_finished_flag.store(true, Ordering::Release);

                Err(Error::from(BootstrapError::StatsdReportingIntervalIsZero))
            },
        );

        drop(service_thread);

        assert!(run_finished.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn waiting_reports_the_error_of_a_failed_run() {
        let service_thread =
            ServiceThread::spawn(CancellationToken::new(), |_task_cancellation_token| async {
                Err(Error::from(BootstrapError::StatsdReportingIntervalIsZero))
            });

        let completion_error = service_thread
            .wait_for_completion()
            .await
            .expect_err("a failed run must be reported to the waiter");

        assert!(matches!(
            completion_error,
            BootstrapError::ServiceRunFailed { source }
                if source.downcast_ref::<BootstrapError>().map(discriminant)
                    == Some(discriminant(&BootstrapError::StatsdReportingIntervalIsZero))
        ));
    }

    #[tokio::test]
    async fn wait_for_completion_errors_when_service_thread_panics() {
        let service_thread =
            ServiceThread::spawn(CancellationToken::new(), |_task_cancellation_token| async {
                panic!("service thread crashed")
            });

        let completion_result = service_thread.wait_for_completion().await;

        assert_eq!(
            completion_result.err().as_ref().map(discriminant),
            Some(discriminant(&BootstrapError::ServiceThreadPanicked))
        );
    }

    #[tokio::test]
    async fn cancel_stops_the_running_service_thread() {
        let service_thread = ServiceThread::spawn(
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
