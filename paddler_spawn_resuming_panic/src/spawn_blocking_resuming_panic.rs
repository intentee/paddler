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

#[cfg(test)]
mod tests {
    use super::spawn_blocking_resuming_panic;

    #[tokio::test]
    async fn the_awaiting_task_receives_what_the_blocking_work_returns() {
        assert_eq!(
            spawn_blocking_resuming_panic(|| "returned").await,
            "returned"
        );
    }

    #[tokio::test]
    #[should_panic(expected = "the blocking work crashed")]
    async fn the_awaiting_task_resumes_the_panic_of_the_blocking_work() {
        spawn_blocking_resuming_panic(|| {
            panic!("the blocking work crashed");
        })
        .await;
    }
}
