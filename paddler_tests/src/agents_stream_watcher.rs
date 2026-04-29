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

    pub async fn wait_for_slots_ready(
        &mut self,
        expected_agent_count: usize,
        slots_per_agent: i32,
    ) -> Result<()> {
        let snapshot = self
            .until(move |snapshot| {
                snapshot.agents.len() >= expected_agent_count
                    && snapshot.agents.iter().all(|agent| {
                        agent.slots_total >= slots_per_agent || !agent.issues.is_empty()
                    })
            })
            .await
            .context("agents did not reach the requested slot count")?;

        let issues: Vec<_> = snapshot
            .agents
            .iter()
            .flat_map(|agent| agent.issues.iter().cloned())
            .collect();

        if !issues.is_empty() {
            bail!("agents reported issues while waiting for slots: {issues:?}");
        }

        Ok(())
    }
}
