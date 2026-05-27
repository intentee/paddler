use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler_cli_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn agent_controller_pool_re_selects_after_contended_claim() -> Result<()> {
    let pool = Arc::new(AgentControllerPool::default());

    let controller_a = Arc::new(make_agent_controller_without_remote_agent("agent-a"));
    controller_a.slots_total.set(4);
    pool.register_agent_controller("agent-a".to_owned(), controller_a)?;

    let controller_b = Arc::new(make_agent_controller_without_remote_agent("agent-b"));
    controller_b.slots_total.set(4);
    pool.register_agent_controller("agent-b".to_owned(), controller_b)?;

    let candidate_first = pool
        .select_least_busy_with_capacity()
        .ok_or_else(|| anyhow!("expected a candidate when both agents have free capacity"))?;
    let first_pick_id = candidate_first.agent_controller.id.clone();

    assert_eq!(
        candidate_first.snapshot, 0,
        "snapshot must capture the value observed at selection time"
    );

    assert!(
        candidate_first
            .agent_controller
            .slots_processing
            .compare_and_swap(0, 1),
        "simulated contender must succeed at incrementing the targeted agent before our claim"
    );

    let claim_outcome = pool.try_claim(candidate_first);

    assert!(
        claim_outcome.is_err(),
        "stale snapshot must produce a Contended (Err) outcome, not a successful claim"
    );

    let candidate_second = pool
        .select_least_busy_with_capacity()
        .ok_or_else(|| anyhow!("expected a candidate after re-selection"))?;

    assert_ne!(
        candidate_second.agent_controller.id, first_pick_id,
        "after a contended claim, re-selection must pick the truly-least-busy agent (the other one)"
    );
    assert_eq!(
        candidate_second.snapshot, 0,
        "the un-contended agent's snapshot must still be 0"
    );

    let dispatched = pool
        .try_claim(candidate_second)
        .map_err(|_| anyhow!("fresh selection must claim the un-contended agent"))?;

    assert_ne!(
        dispatched.agent_controller.id, first_pick_id,
        "the dispatched agent must be the one selected after re-selection"
    );

    Ok(())
}
