use paddler_agent::reconciled_state::ReconciledState;
use paddler_agent::reconciled_state_holder::ReconciledStateHolder;
use paddler_agent_tests::state_overriding_the_chat_template::state_overriding_the_chat_template;
use paddler_messaging::desired_state_version::DesiredStateVersion;

#[test]
fn a_new_subscriber_sees_the_state_reconciled_before_it_subscribed() {
    let holder = ReconciledStateHolder::default();
    let reconciled_state = ReconciledState {
        agent_applicable_state: state_overriding_the_chat_template(),
        desired_state_version: DesiredStateVersion::Version(1),
    };

    holder.set_reconciled_state(reconciled_state.clone());

    let reconciled_state_rx = holder.subscribe();

    assert!(reconciled_state_rx.has_changed().unwrap());
    assert_eq!(*reconciled_state_rx.borrow(), reconciled_state);
}
