use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent_tests::arbiter_under_test::ArbiterUnderTest;
use paddler_agent_tests::state_overriding_the_chat_template::state_overriding_the_chat_template;
use paddler_agent_tests::state_whose_model_cannot_load::state_whose_model_cannot_load;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

#[tokio::test]
async fn a_replacement_whose_model_cannot_load_reports_nothing_as_served() {
    let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

    arbiter_under_test
        .pipeline_arbiter
        .apply(state_overriding_the_chat_template());
    arbiter_under_test.conclude_next_progress().await;

    assert_eq!(
        arbiter_under_test.conclude_next_progress().await,
        AgentStateApplicationStatus::Applied
    );

    arbiter_under_test
        .pipeline_arbiter
        .apply(state_whose_model_cannot_load());
    arbiter_under_test.conclude_next_progress().await;

    assert_eq!(
        arbiter_under_test.conclude_next_progress().await,
        AgentStateApplicationStatus::AttemptedAndRetrying
    );
    assert_eq!(
        arbiter_under_test
            .served_applicable_state_holder
            .get_agent_applicable_state(),
        AgentApplicableState::NotConfigured
    );

    arbiter_under_test.pipeline_arbiter.shut_down().await;
}
