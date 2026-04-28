use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::anyhow;
use futures_util::stream;
use paddler_tests::agents_status::assert_slots_total_at_least::assert_slots_total_at_least;
use paddler_tests::agents_stream_watcher::AgentsStreamWatcher;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

fn make_snapshot(agent_id: &str, slots_total: i32) -> AgentControllerPoolSnapshot {
    AgentControllerPoolSnapshot {
        agents: vec![AgentControllerSnapshot {
            desired_slots_total: slots_total,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: agent_id.to_owned(),
            issues: BTreeSet::new(),
            model_path: None,
            name: None,
            slots_processing: 0,
            slots_total,
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: false,
        }],
    }
}

#[tokio::test]
async fn until_returns_first_snapshot_matching_predicate() -> Result<()> {
    let fixture = stream::iter(vec![
        Ok(make_snapshot("agent-a", 0)),
        Ok(make_snapshot("agent-a", 1)),
        Ok(make_snapshot("agent-a", 4)),
    ]);

    let mut watcher = AgentsStreamWatcher::from_stream(Box::pin(fixture));

    let snapshot = watcher
        .until(assert_slots_total_at_least("agent-a", 1))
        .await?;

    assert_eq!(snapshot.agents.len(), 1);
    assert_eq!(snapshot.agents[0].slots_total, 1);

    Ok(())
}

#[tokio::test]
async fn until_propagates_stream_error() {
    let fixture = stream::iter(vec![Err(anyhow!(
        "simulated SSE failure from upstream server"
    ))]);

    let mut watcher = AgentsStreamWatcher::from_stream(Box::pin(fixture));

    let outcome = watcher.until(|_| true).await;

    assert!(outcome.is_err(), "expected watcher to surface stream error");

    let error_chain = format!(
        "{:#}",
        outcome.err().unwrap_or_else(|| anyhow!("unreachable"))
    );

    assert!(
        error_chain.contains("simulated SSE failure from upstream server"),
        "expected original error message in chain, got: {error_chain}"
    );
}

#[tokio::test]
async fn until_errors_when_stream_closes_before_match() {
    let fixture = stream::iter(vec![Ok(make_snapshot("agent-a", 0))]);

    let mut watcher = AgentsStreamWatcher::from_stream(Box::pin(fixture));

    let outcome = watcher
        .until(assert_slots_total_at_least("agent-a", 10))
        .await;

    assert!(
        outcome.is_err(),
        "expected error when stream closes without satisfying predicate"
    );
}
