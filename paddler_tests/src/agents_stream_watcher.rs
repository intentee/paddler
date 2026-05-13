use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::ClientManagement;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub struct AgentsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<AgentControllerPoolSnapshot>> + Send>>,
}

impl AgentsStreamWatcher {
    pub async fn connect(management: &ClientManagement<'_>) -> Result<Self> {
        let raw_stream = management
            .get_agents_stream()
            .await
            .map_err(anyhow::Error::new)
            .context("failed to open /api/v1/agents/stream")?;

        let stream = raw_stream.map(|item| item.map_err(anyhow::Error::new));

        Ok(Self {
            stream: Box::pin(stream),
        })
    }

    #[must_use]
    pub fn from_stream(
        stream: Pin<Box<dyn Stream<Item = Result<AgentControllerPoolSnapshot>> + Send>>,
    ) -> Self {
        Self { stream }
    }

    pub async fn until<TPredicate>(
        &mut self,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        while let Some(item) = self.stream.next().await {
            let snapshot = item.context("agents stream yielded an error")?;

            if predicate(&snapshot) {
                return Ok(snapshot);
            }
        }

        Err(anyhow!(
            "agents stream closed before predicate was satisfied"
        ))
    }

    pub async fn until_agent<TPredicate>(
        &mut self,
        agent_id: &str,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        while let Some(item) = self.stream.next().await {
            let snapshot = item.context("agents stream yielded an error")?;

            let agent_present = snapshot
                .agents
                .iter()
                .any(|registered_agent| registered_agent.id == agent_id);

            if !agent_present {
                bail!(
                    "agent {agent_id} disappeared from the balancer's agent pool before the predicate was satisfied; this means the agent subprocess died or its WebSocket dropped"
                );
            }

            if predicate(&snapshot) {
                return Ok(snapshot);
            }
        }

        Err(anyhow!(
            "agents stream closed before predicate was satisfied"
        ))
    }

    pub async fn wait_for_slots_ready(&mut self, expected_slot_counts: &[i32]) -> Result<()> {
        let mut expected_sorted: Vec<i32> = expected_slot_counts.to_vec();
        expected_sorted.sort_unstable();
        let expected_agent_count = expected_sorted.len();

        let snapshot = self
            .until(move |snapshot| {
                if snapshot.agents.len() < expected_agent_count {
                    return false;
                }

                let any_with_issues = snapshot.agents.iter().any(|agent| !agent.issues.is_empty());

                if any_with_issues {
                    return true;
                }

                let mut observed_slot_counts: Vec<i32> = snapshot
                    .agents
                    .iter()
                    .map(|agent| agent.slots_total)
                    .collect();
                observed_slot_counts.sort_unstable();

                observed_slot_counts == expected_sorted
            })
            .await
            .context("agents did not reach the requested slot counts")?;

        let agents_with_issues: Vec<String> = snapshot
            .agents
            .iter()
            .filter(|agent| !agent.issues.is_empty())
            .map(|agent| format!("agent {}: {:?}", agent.id, agent.issues))
            .collect();

        if !agents_with_issues.is_empty() {
            bail!(
                "agents reported issues while waiting for slots: {}",
                agents_with_issues.join("; ")
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use futures_util::stream;
    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_issue::AgentIssue;
    use paddler_types::agent_issue_params::ModelPath;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

    use super::*;

    fn snapshot_with_agent(
        agent_id: &str,
        issues: BTreeSet<AgentIssue>,
    ) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            desired_slots_total: 1,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: agent_id.to_owned(),
            issues,
            model_path: None,
            name: Some(agent_id.to_owned()),
            slots_processing: 0,
            slots_total: 0,
            state_application_status: AgentStateApplicationStatus::Fresh,
            uses_chat_template_override: false,
        }
    }

    fn unable_to_find_chat_template_issue() -> BTreeSet<AgentIssue> {
        let mut issues = BTreeSet::new();
        issues.insert(AgentIssue::UnableToFindChatTemplate(ModelPath {
            model_path: "/models/embed.gguf".to_owned(),
        }));
        issues
    }

    fn make_watcher(snapshots: Vec<AgentControllerPoolSnapshot>) -> AgentsStreamWatcher {
        AgentsStreamWatcher::from_stream(Box::pin(stream::iter(snapshots.into_iter().map(Ok))))
    }

    #[tokio::test]
    async fn until_agent_returns_ok_when_predicate_matches_with_agent_present() -> Result<()> {
        let agent_id = "agent-x";
        let snapshots = vec![
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent(agent_id, BTreeSet::new())],
            },
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent(
                    agent_id,
                    unable_to_find_chat_template_issue(),
                )],
            },
        ];

        let mut watcher = make_watcher(snapshots);

        let predicate_agent_id = agent_id.to_owned();
        let observed = watcher
            .until_agent(agent_id, move |snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent.id == predicate_agent_id
                        && agent
                            .issues
                            .iter()
                            .any(|issue| matches!(issue, AgentIssue::UnableToFindChatTemplate(_)))
                })
            })
            .await?;

        assert!(
            observed.agents.iter().any(|agent| agent.id == agent_id),
            "matched snapshot must contain the watched agent"
        );

        Ok(())
    }

    #[tokio::test]
    async fn until_agent_errors_when_agent_disappears_mid_stream() -> Result<()> {
        let agent_id = "agent-y";
        let snapshots = vec![
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent(agent_id, BTreeSet::new())],
            },
            AgentControllerPoolSnapshot { agents: vec![] },
        ];

        let mut watcher = make_watcher(snapshots);

        let error = watcher
            .until_agent(agent_id, |_snapshot| false)
            .await
            .err()
            .context("until_agent must surface the disappearance as an error")?;
        let rendered = format!("{error:#}");

        assert!(
            rendered.contains("disappeared"),
            "error must explicitly call out the disappearance, got: {rendered}"
        );
        assert!(
            rendered.contains(agent_id),
            "error must name the missing agent, got: {rendered}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn until_agent_errors_when_stream_closes_before_predicate_matches() -> Result<()> {
        let agent_id = "agent-z";
        let snapshots = vec![AgentControllerPoolSnapshot {
            agents: vec![snapshot_with_agent(agent_id, BTreeSet::new())],
        }];

        let mut watcher = make_watcher(snapshots);

        let error = watcher
            .until_agent(agent_id, |_snapshot| false)
            .await
            .err()
            .context("until_agent must error when the stream ends without a match")?;
        let rendered = format!("{error:#}");

        assert!(
            rendered.contains("stream closed"),
            "error must surface the stream-closed condition, got: {rendered}"
        );

        Ok(())
    }
}
