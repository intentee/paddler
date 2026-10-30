use std::panic::resume_unwind;
use std::thread::JoinHandle;

pub fn join_scheduler_thread(scheduler_thread_handle: JoinHandle<()>) {
    scheduler_thread_handle
        .join()
        .unwrap_or_else(|panic_payload| resume_unwind(panic_payload));
}
