use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;

#[test]
fn agent_controller_pool_signals_update_when_slot_guard_drops() -> Result<()> {
    let pool = AgentControllerPool::default();
    let controller = Arc::new(make_agent_controller_without_remote_agent("test-agent"));

    controller.slots_total.set(1);

    pool.register_agent_controller("test-agent".to_owned(), controller)
        .context("agent registration must succeed")?;

    let mut update_rx = pool.subscribe_to_updates();

    update_rx.borrow_and_update();

    let dispatched = pool
        .take_least_busy_agent_controller()
        .ok_or_else(|| anyhow!("a free slot must be available"))?;

    update_rx.borrow_and_update();

    drop(dispatched);

    if !update_rx.has_changed().context("watch sender dropped")? {
        return Err(anyhow!(
            "slot guard drop must notify pool update subscribers"
        ));
    }

    Ok(())
}
