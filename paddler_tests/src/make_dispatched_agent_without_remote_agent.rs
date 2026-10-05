use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use paddler_balancer::agent_controller::AgentController;
use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::dispatched_agent::DispatchedAgent;

pub fn make_dispatched_agent_without_remote_agent(
    agent_controller: Arc<AgentController>,
) -> Result<DispatchedAgent> {
    let pool = AgentControllerPool::default();

    agent_controller.slots_total.set(1);

    pool.register_agent_controller(agent_controller.id.clone(), agent_controller)?;
    pool.take_least_busy_agent_controller()
        .context("a freshly registered agent controller must have a free slot")
}
