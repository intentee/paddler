use std::sync::Arc;

use anyhow::Result;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use tokio::sync::Barrier;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn agent_controller_pool_does_not_oversubscribe_under_concurrent_dispatch() -> Result<()> {
    const PARALLEL_CALLERS: usize = 256;

    let pool = Arc::new(AgentControllerPool::default());
    let controller = Arc::new(make_agent_controller_without_remote_agent("solo-agent"));
    controller.slots_total.set(1);
    pool.register_agent_controller("solo-agent".to_owned(), controller)?;

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

    assert!(
        acquired.len() <= 1,
        "take_least_busy_agent_controller oversubscribed: {} callers held an agent simultaneously with slots_total=1",
        acquired.len()
    );

    Ok(())
}
