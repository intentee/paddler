use std::panic::resume_unwind;

use tokio::spawn;

pub async fn spawn_resuming_panic<TWork>(work: TWork) -> TWork::Output
where
    TWork: Future + Send + 'static,
    TWork::Output: Send + 'static,
{
    spawn(work)
        .await
        .unwrap_or_else(|join_error| resume_unwind(join_error.into_panic()))
}

#[cfg(test)]
mod tests {
    use super::spawn_resuming_panic;

    #[tokio::test]
    async fn the_awaiting_task_receives_what_the_spawned_task_returns() {
        assert_eq!(spawn_resuming_panic(async { "returned" }).await, "returned");
    }

    #[tokio::test]
    #[should_panic(expected = "the spawned task crashed")]
    async fn the_awaiting_task_resumes_the_panic_of_the_spawned_task() {
        spawn_resuming_panic(async {
            panic!("the spawned task crashed");
        })
        .await;
    }
}
