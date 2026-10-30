use std::thread::JoinHandle;

use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::join_scheduler_thread::join_scheduler_thread;

pub async fn join_scheduler_thread_in_background<TReleasedFirst>(
    released_first: TReleasedFirst,
    scheduler_thread_handle: JoinHandle<()>,
) where
    TReleasedFirst: Send + 'static,
{
    spawn_blocking_resuming_panic(move || {
        drop(released_first);
        join_scheduler_thread(scheduler_thread_handle);
    })
    .await;
}
