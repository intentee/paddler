use paddler_agent::arbitrated_state::ArbitratedState;
use paddler_agent_tests::state_overriding_the_chat_template::state_overriding_the_chat_template;

#[test]
fn an_identical_state_is_not_arbitrated_again() {
    assert!(
        ArbitratedState::Arbitrating(state_overriding_the_chat_template())
            .arbitrates(&state_overriding_the_chat_template())
    );
}
