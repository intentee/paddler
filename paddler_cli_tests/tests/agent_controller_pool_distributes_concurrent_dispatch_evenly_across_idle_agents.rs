use std::sync::Arc;

use anyhow::Result;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler_cli_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use tokio::sync::Barrier;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn agent_controller_pool_distributes_concurrent_dispatch_evenly_across_idle_agents()
-> Result<()> {
    const AGENT_COUNT: usize = 4;
    const SLOTS_PER_AGENT: i32 = 4;
    const PARALLEL_CALLERS: usize = AGENT_COUNT * (SLOTS_PER_AGENT as usize);

    let pool = Arc::new(AgentControllerPool::default());
    let mut controllers = Vec::with_capacity(AGENT_COUNT);

    for index in 0..AGENT_COUNT {
        let agent_id = format!("agent-{index}");
        let controller = Arc::new(make_agent_controller_without_remote_agent(&agent_id));

        controller.slots_total.set(SLOTS_PER_AGENT);
        pool.register_agent_controller(agent_id, controller.clone())?;
        controllers.push(controller);
    }

    let barrier = Arc::new(Barrier::new(PARALLEL_CALLERS));
    let mut handles = Vec::with_capacity(PARALLEL_CALLERS);

    for _ in 0..PARALLEL_CALLERS {
        let pool_for_task = pool.clone();
        let barrier_for_task = barrier.clone();

        handles.push(tokio::spawn(async move {
            barrier_for_task.wait().await;

            pool_for_task.take_least_busy_agent_controller()
        }));
    }

    let mut acquired = Vec::with_capacity(PARALLEL_CALLERS);

    for handle in handles {
        if let Some(dispatched_agent) = handle.await? {
            acquired.push(dispatched_agent);
        }
    }

    assert_eq!(
        acquired.len(),
        PARALLEL_CALLERS,
        "every caller must acquire a slot when total capacity equals concurrency"
    );

    for controller in &controllers {
        assert_eq!(
            controller.slots_processing.get(),
            SLOTS_PER_AGENT,
            "agent {} should be filled to capacity under fair burst dispatch",
            controller.id,
        );
    }

    Ok(())
}
