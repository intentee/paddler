use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::anyhow;
use futures_util::stream;
use paddler_cluster::agents_stream_watcher::AgentsStreamWatcher;
use paddler_cluster::error::ClusterError;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

fn make_snapshot(agent_id: &str, slots_total: i32) -> AgentControllerPoolSnapshot {
    AgentControllerPoolSnapshot {
        agents: vec![AgentControllerSnapshot {
            desired_slots_total: slots_total,
            download_current: 0,
            download_filename: None,
            download_indeterminate: true,
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
        .until(|snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == "agent-a" && agent.slots_total >= 1)
        })
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

    assert!(matches!(
        outcome,
        Err(ClusterError::SnapshotStreamYielded { .. })
    ));
}

#[tokio::test]
async fn until_errors_when_stream_closes_before_match() {
    let fixture = stream::iter(vec![Ok(make_snapshot("agent-a", 0))]);

    let mut watcher = AgentsStreamWatcher::from_stream(Box::pin(fixture));

    let outcome = watcher
        .until(|snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == "agent-a" && agent.slots_total >= 10)
        })
        .await;

    assert!(matches!(outcome, Err(ClusterError::SnapshotStreamClosed)));
}

#[tokio::test]
async fn wait_for_slots_ready_includes_agent_id_in_error() -> Result<()> {
    let mut snapshot = make_snapshot("agent-x", 0);
    let mut issues = BTreeSet::new();
    issues.insert(AgentIssue::ModelFileDoesNotExist(ModelPath {
        model_path: "/nonexistent".to_owned(),
    }));
    snapshot.agents[0].issues = issues;

    let fixture = stream::iter(vec![Ok(snapshot)]);
    let mut watcher = AgentsStreamWatcher::from_stream(Box::pin(fixture));

    let outcome = watcher.wait_for_slots_ready(&[1]).await;

    match outcome {
        Err(ClusterError::AgentReportedIssues { agent_id, .. }) => {
            assert_eq!(agent_id.as_str(), "agent-x");

            Ok(())
        }
        other => Err(anyhow!("expected AgentReportedIssues, got {other:?}")),
    }
}
