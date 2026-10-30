use tokio_util::sync::CancellationToken;

use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

use crate::arbiter_under_test::ArbiterUnderTest;
use crate::state_whose_model_cannot_load::state_whose_model_cannot_load;

#[tokio::test]
async fn a_startup_cancelled_by_the_agent_shutting_down_changes_nothing() {
    let agent_shutdown = CancellationToken::new();
    let mut arbiter_under_test = ArbiterUnderTest::new(agent_shutdown.clone());

    agent_shutdown.cancel();
    arbiter_under_test
        .pipeline_arbiter
        .apply(state_whose_model_cannot_load());

    let status_before_the_startup_concluded = arbiter_under_test
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot();

    arbiter_under_test.conclude_next_progress().await;

    assert_eq!(
        arbiter_under_test
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot(),
        status_before_the_startup_concluded
    );

    arbiter_under_test.pipeline_arbiter.shut_down().await;
}
