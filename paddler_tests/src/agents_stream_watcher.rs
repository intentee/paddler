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
