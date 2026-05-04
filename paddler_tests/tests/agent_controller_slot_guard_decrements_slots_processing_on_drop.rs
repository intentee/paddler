use std::sync::Arc;

use paddler::balancer::agent_controller_slot_guard::AgentControllerSlotGuard;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use tokio::sync::watch;

#[test]
fn agent_controller_slot_guard_decrements_slots_processing_on_drop() {
    let controller = Arc::new(make_agent_controller_without_remote_agent("test-agent"));
    controller.slots_total.set(2);
    controller.slots_processing.increment();

    assert_eq!(controller.slots_processing.get(), 1);

    let (pool_update_tx, _pool_update_rx) = watch::channel(());
    let guard = AgentControllerSlotGuard::new(controller.clone(), pool_update_tx);

    drop(guard);

    assert_eq!(controller.slots_processing.get(), 0);
}
