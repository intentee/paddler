use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use tokio_util::sync::CancellationToken;

pub struct AgentsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<AgentControllerPoolSnapshot>> + Send>>,
}

impl AgentsStreamWatcher {
    pub async fn connect(
        cancellation_token: CancellationToken,
        management: &ClientManagement,
    ) -> Result<Self> {
        let raw_stream = management
            .get_agents_stream(cancellation_token)
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

    pub async fn wait_for_agent_ready(
        &mut self,
        agent_name: &str,
        expected_slot_count: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        let predicate_name = agent_name.to_owned();
        let snapshot = self
            .until(move |snapshot| {
                snapshot.agents.iter().any(|registered_agent| {
                    registered_agent.name.as_deref() == Some(predicate_name.as_str())
                        && (registered_agent.slots_total == expected_slot_count
                            || !registered_agent.issues.is_empty())
                })
            })
            .await
            .with_context(|| format!("agent {agent_name:?} did not reach slot readiness"))?;

        let agent_with_issues = snapshot.agents.iter().find(|registered_agent| {
            registered_agent.name.as_deref() == Some(agent_name)
                && !registered_agent.issues.is_empty()
        });

        if let Some(failing_agent) = agent_with_issues {
            bail!(
                "agent {agent_name:?} reported issues during startup: {issues:?}",
                issues = failing_agent.issues,
            );
        }

        Ok(snapshot)
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
    use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

    use super::*;

    fn snapshot_with_agent(
        agent_id: &str,
        issues: BTreeSet<AgentIssue>,
    ) -> AgentControllerSnapshot {
        snapshot_with_agent_and_slots(agent_id, issues, 0)
    }

    fn snapshot_with_agent_and_slots(
        agent_id: &str,
        issues: BTreeSet<AgentIssue>,
        slots_total: i32,
    ) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            desired_slots_total: 1,
            download_current: 0,
            download_filename: None,
            download_indeterminate: true,
            download_total: 0,
            id: agent_id.to_owned(),
            issues,
            model_path: None,
            name: Some(agent_id.to_owned()),
            slots_processing: 0,
            slots_total,
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

    #[tokio::test]
    async fn wait_for_agent_ready_returns_snapshot_when_named_agent_reaches_slot_count()
    -> Result<()> {
        let agent_id = "agent-warm-0";
        let snapshots = vec![
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent_and_slots(agent_id, BTreeSet::new(), 0)],
            },
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent_and_slots(agent_id, BTreeSet::new(), 2)],
            },
        ];

        let mut watcher = make_watcher(snapshots);

        let snapshot = watcher.wait_for_agent_ready(agent_id, 2).await?;

        assert!(
            snapshot
                .agents
                .iter()
                .any(|agent| { agent.name.as_deref() == Some(agent_id) && agent.slots_total == 2 }),
            "returned snapshot must contain the named agent at its target slot count"
        );

        Ok(())
    }

    #[tokio::test]
    async fn wait_for_agent_ready_errors_when_named_agent_reports_issues() -> Result<()> {
        let agent_id = "agent-warm-1";
        let snapshots = vec![AgentControllerPoolSnapshot {
            agents: vec![snapshot_with_agent_and_slots(
                agent_id,
                unable_to_find_chat_template_issue(),
                0,
            )],
        }];

        let mut watcher = make_watcher(snapshots);

        let error = watcher
            .wait_for_agent_ready(agent_id, 2)
            .await
            .err()
            .context("wait_for_agent_ready must surface agent-side issues as an error")?;
        let rendered = format!("{error:#}");

        assert!(
            rendered.contains(agent_id),
            "error must name the failing agent, got: {rendered}"
        );
        assert!(
            rendered.contains("issues"),
            "error must mention that issues were registered, got: {rendered}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn wait_for_agent_ready_errors_when_stream_closes_before_match() -> Result<()> {
        let agent_id = "agent-warm-2";
        let snapshots = vec![AgentControllerPoolSnapshot {
            agents: vec![snapshot_with_agent_and_slots(agent_id, BTreeSet::new(), 0)],
        }];

        let mut watcher = make_watcher(snapshots);

        let error = watcher
            .wait_for_agent_ready(agent_id, 2)
            .await
            .err()
            .context("wait_for_agent_ready must error when the stream ends without a match")?;
        let rendered = format!("{error:#}");

        assert!(
            rendered.contains("slot readiness"),
            "error must mention that slot readiness was not reached, got: {rendered}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn wait_for_slots_ready_completes_once_observed_counts_match_expected() {
        let snapshots = vec![
            AgentControllerPoolSnapshot {
                agents: vec![snapshot_with_agent_and_slots("a", BTreeSet::new(), 1)],
            },
            AgentControllerPoolSnapshot {
                agents: vec![
                    snapshot_with_agent_and_slots("a", BTreeSet::new(), 1),
                    snapshot_with_agent_and_slots("b", BTreeSet::new(), 2),
                ],
            },
        ];

        let mut watcher = make_watcher(snapshots);

        watcher.wait_for_slots_ready(&[2, 1]).await.unwrap();
    }

    #[tokio::test]
    async fn wait_for_slots_ready_errors_when_an_agent_reports_issues() {
        let snapshots = vec![AgentControllerPoolSnapshot {
            agents: vec![
                snapshot_with_agent_and_slots("a", unable_to_find_chat_template_issue(), 1),
                snapshot_with_agent_and_slots("b", BTreeSet::new(), 2),
            ],
        }];

        let mut watcher = make_watcher(snapshots);

        let error = watcher.wait_for_slots_ready(&[1, 2]).await.err().unwrap();

        assert!(format!("{error:#}").contains("issues"));
    }
}
