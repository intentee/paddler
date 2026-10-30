use tokio::task::spawn_blocking;

use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;

#[test]
fn a_dropped_occupied_blocking_thread_lets_the_next_blocking_task_run() {
    single_blocking_thread_runtime()
        .expect("the test runtime must build")
        .block_on(async {
            drop(OccupiedBlockingThread::occupy());

            assert_eq!(spawn_blocking(|| ()).await.ok(), Some(()));
        });
}
