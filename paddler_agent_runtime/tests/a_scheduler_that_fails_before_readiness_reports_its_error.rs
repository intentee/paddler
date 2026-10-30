use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;

use crate::even_number_preparation::EvenNumberPreparation;

#[tokio::test]
async fn a_scheduler_that_fails_before_readiness_reports_its_error() {
    let startup_result = start_scheduler_thread::<EvenNumberPreparation, String, _>(
        &CancellationToken::new(),
        |_agent_shutdown, _scheduler_startup_tx| Err("the model could not be loaded".to_owned()),
    )
    .await;

    assert_eq!(
        startup_result.err(),
        Some("the model could not be loaded".to_owned())
    );
}
