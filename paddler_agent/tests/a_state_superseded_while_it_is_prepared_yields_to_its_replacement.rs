use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;

use crate::arbiter_under_test::ArbiterUnderTest;
use crate::state_overriding_the_chat_template::state_overriding_the_chat_template;

#[tokio::test]
async fn a_state_superseded_while_it_is_prepared_yields_to_its_replacement() {
    let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

    arbiter_under_test
        .pipeline_arbiter
        .apply(AgentApplicableState::NotConfigured);
    arbiter_under_test
        .pipeline_arbiter
        .apply(state_overriding_the_chat_template());
    arbiter_under_test.conclude_next_progress().await;
    arbiter_under_test.conclude_next_progress().await;
    arbiter_under_test.conclude_next_progress().await;

    assert_eq!(
        arbiter_under_test
            .served_applicable_state_holder
            .get_agent_applicable_state(),
        state_overriding_the_chat_template()
    );

    arbiter_under_test.pipeline_arbiter.shut_down().await;
}
