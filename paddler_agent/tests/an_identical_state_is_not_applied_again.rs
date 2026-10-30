use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;

use crate::state_overriding_the_chat_template::state_overriding_the_chat_template;

#[test]
fn an_identical_state_is_not_applied_again() {
    let holder = AgentApplicableStateHolder::default();
    let applied_state = state_overriding_the_chat_template();

    holder.set_agent_applicable_state(applied_state.clone());

    let mut agent_applicable_state_rx = holder.subscribe();

    agent_applicable_state_rx.borrow_and_update();
    holder.set_agent_applicable_state(applied_state);

    assert!(!agent_applicable_state_rx.has_changed().unwrap());
}
