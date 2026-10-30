use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

use crate::arbiter_under_test::ArbiterUnderTest;
use crate::state_whose_model_cannot_load::state_whose_model_cannot_load;

#[tokio::test]
async fn a_pipeline_that_fails_to_start_is_retried_until_it_is_stuck() {
    let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

    arbiter_under_test
        .pipeline_arbiter
        .apply(state_whose_model_cannot_load());

    assert_eq!(
        [
            arbiter_under_test.conclude_next_progress().await,
            arbiter_under_test.conclude_next_progress().await,
            arbiter_under_test.conclude_next_progress().await,
            arbiter_under_test.conclude_next_progress().await,
        ],
        [
            AgentStateApplicationStatus::Fresh,
            AgentStateApplicationStatus::AttemptedAndRetrying,
            AgentStateApplicationStatus::AttemptedAndRetrying,
            AgentStateApplicationStatus::Stuck,
        ]
    );

    arbiter_under_test.pipeline_arbiter.shut_down().await;
}
