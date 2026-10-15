use std::panic::resume_unwind;

use tokio::task::spawn_blocking;

pub async fn spawn_blocking_resuming_panic<TBlockingWork, TOutput>(
    blocking_work: TBlockingWork,
) -> TOutput
where
    TBlockingWork: FnOnce() -> TOutput + Send + 'static,
    TOutput: Send + 'static,
{
    spawn_blocking(blocking_work)
        .await
        .unwrap_or_else(|join_error| resume_unwind(join_error.into_panic()))
}
