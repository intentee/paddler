use std::thread::spawn;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;

use crate::even_number_preparation::EvenNumberPreparation;

#[tokio::test]
async fn a_scheduler_that_fails_before_readiness_reports_its_error() {
    let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();
    let scheduler_thread_handle = spawn(move || {
        drop(scheduler_ready_tx);

        Err("the model could not be loaded".to_owned())
    });

    let startup_result = await_scheduler_startup::<EvenNumberPreparation, String>(
        &CancellationToken::new(),
        scheduler_ready_rx,
        scheduler_thread_handle,
    )
    .await;

    assert_eq!(
        startup_result.err(),
        Some("the model could not be loaded".to_owned())
    );
}
