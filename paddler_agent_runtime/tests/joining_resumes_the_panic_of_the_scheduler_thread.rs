use std::thread::spawn;

use paddler_agent_runtime::join_scheduler_thread::join_scheduler_thread;

#[test]
#[should_panic(expected = "the scheduler crashed")]
fn joining_resumes_the_panic_of_the_scheduler_thread() {
    let scheduler_thread_handle = spawn(|| -> Result<(), String> {
        panic!("the scheduler crashed");
    });

    drop(join_scheduler_thread(scheduler_thread_handle));
}
