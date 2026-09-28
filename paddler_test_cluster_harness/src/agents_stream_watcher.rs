use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::observation_window::ObservationWindow;

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

    pub async fn until<TPredicate>(
        &mut self,
        observation_window: ObservationWindow,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        let stream = &mut self.stream;

        timeout(observation_window.duration(), async move {
            while let Some(item) = stream.next().await {
                let snapshot = item.context("agents stream yielded an error")?;

                if predicate(&snapshot) {
                    return Ok(snapshot);
                }
            }

            Err(anyhow!(
                "agents stream closed before predicate was satisfied"
            ))
        })
        .await
        .with_context(|| {
            format!(
                "agents stream did not satisfy the predicate within {:?}",
                observation_window.duration()
            )
        })?
    }

    pub async fn until_agent<TPredicate>(
        &mut self,
        agent_id: &str,
        observation_window: ObservationWindow,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        let stream = &mut self.stream;

        timeout(observation_window.duration(), async move {
            while let Some(item) = stream.next().await {
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
        })
        .await
        .with_context(|| {
            format!(
                "agent {agent_id} did not satisfy the predicate within {:?}",
                observation_window.duration()
            )
        })?
    }

    pub async fn wait_for_agent_ready(
        &mut self,
        agent_name: &str,
        expected_slot_count: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        let predicate_name = agent_name.to_owned();
        let snapshot = self
            .until(ObservationWindow::model_load(), move |snapshot| {
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
            .until(ObservationWindow::model_load(), move |snapshot| {
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
