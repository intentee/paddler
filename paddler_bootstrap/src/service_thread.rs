use std::future::Future;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;
use log::error;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub struct ServiceThread {
    completion_rx: Option<oneshot::Receiver<Result<()>>>,
    shutdown: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}

impl ServiceThread {
    pub fn spawn<TRun, TFuture>(parent_shutdown: Option<CancellationToken>, run: TRun) -> Self
    where
        TRun: FnOnce(CancellationToken) -> TFuture + Send + 'static,
        TFuture: Future<Output = Result<()>>,
    {
        let shutdown =
            parent_shutdown.map_or_else(CancellationToken::new, |parent| parent.child_token());
        let task_shutdown = shutdown.clone();
        let (completion_tx, completion_rx) = oneshot::channel::<Result<()>>();

        let thread = thread::spawn(move || {
            let result = actix_web::rt::System::new().block_on(run(task_shutdown));
            let _ = completion_tx.send(result);
        });

        Self {
            completion_rx: Some(completion_rx),
            shutdown,
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
        self.shutdown.cancel();
    }
}

impl Drop for ServiceThread {
    fn drop(&mut self) {
        self.shutdown.cancel();

        if let Some(thread) = self.thread.take()
            && let Err(panic_payload) = thread.join()
        {
            error!("service thread panicked: {panic_payload:?}");
        }
    }
}
