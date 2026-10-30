use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;

use crate::state_overriding_the_chat_template::state_overriding_the_chat_template;

#[test]
fn a_new_subscriber_sees_the_state_applied_before_it_subscribed() {
    let holder = AgentApplicableStateHolder::default();
    let applied_state = state_overriding_the_chat_template();

    holder.set_agent_applicable_state(applied_state.clone());

    let agent_applicable_state_rx = holder.subscribe();

    assert!(agent_applicable_state_rx.has_changed().unwrap());
    assert_eq!(*agent_applicable_state_rx.borrow(), applied_state);
}
