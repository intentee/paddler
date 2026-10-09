use std::thread::spawn;

use paddler_agent_runtime::join_scheduler_thread_in_background::join_scheduler_thread_in_background;

#[tokio::test]
#[should_panic(expected = "the scheduler crashed")]
async fn joining_in_the_background_resumes_the_panic_of_the_scheduler_thread() {
    let scheduler_thread_handle = spawn(|| -> Result<(), String> {
        panic!("the scheduler crashed");
    });

    drop(join_scheduler_thread_in_background((), scheduler_thread_handle).await);
}
