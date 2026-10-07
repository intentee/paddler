use std::panic::resume_unwind;
use std::thread::JoinHandle;

use tokio::task::spawn_blocking;

use crate::join_scheduler_thread::join_scheduler_thread;

pub async fn join_scheduler_thread_in_background<TReleasedFirst, TError>(
    released_first: TReleasedFirst,
    scheduler_thread_handle: JoinHandle<Result<(), TError>>,
) -> Result<(), TError>
where
    TReleasedFirst: Send + 'static,
    TError: Send + 'static,
{
    spawn_blocking(move || {
        drop(released_first);

        join_scheduler_thread(scheduler_thread_handle)
    })
    .await
    .unwrap_or_else(|join_error| resume_unwind(join_error.into_panic()))
}
