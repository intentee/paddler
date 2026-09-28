use std::pin::Pin;

use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::agents_stream::AgentsStream;
use paddler_client::buffered_requests_stream::BufferedRequestsStream;
use paddler_client::error::Result as ClientResult;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use tokio::time::timeout;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::observation_window::ObservationWindow;
use crate::snapshots_stream::SnapshotsStream;

pub struct SnapshotsWatcher<TSnapshot> {
    snapshots_stream: SnapshotsStream,
    stream: Pin<Box<dyn Stream<Item = ClientResult<TSnapshot>> + Send>>,
}

impl<TSnapshot> SnapshotsWatcher<TSnapshot> {
    pub async fn until<TPredicate>(
        &mut self,
        observation_window: ObservationWindow,
        mut predicate: TPredicate,
    ) -> Result<TSnapshot, ClusterHarnessError>
    where
        TPredicate: FnMut(&TSnapshot) -> bool,
    {
        self.until_verdict(observation_window, |snapshot| Ok(predicate(snapshot)))
            .await
    }

    async fn until_verdict<TVerdict>(
        &mut self,
        observation_window: ObservationWindow,
        mut verdict: TVerdict,
    ) -> Result<TSnapshot, ClusterHarnessError>
    where
        TVerdict: FnMut(&TSnapshot) -> Result<bool, ClusterHarnessError>,
    {
        let snapshots_stream = self.snapshots_stream;
        let stream = &mut self.stream;

        timeout(observation_window.duration(), async move {
            while let Some(item) = stream.next().await {
                let snapshot =
                    item.map_err(|source| ClusterHarnessError::SnapshotsStreamFailed {
                        snapshots_stream,
                        source,
                    })?;

                if verdict(&snapshot)? {
                    return Ok(snapshot);
                }
            }

            Err(ClusterHarnessError::SnapshotsStreamClosed { snapshots_stream })
        })
        .await
        .map_err(|source| ClusterHarnessError::ObservationWindowElapsed {
            observation_window,
            snapshots_stream,
            source,
        })?
    }
}

impl SnapshotsWatcher<AgentControllerPoolSnapshot> {
    #[must_use]
    pub fn of_agents(stream: AgentsStream) -> Self {
        Self {
            snapshots_stream: SnapshotsStream::Agents,
            stream,
        }
    }

    pub async fn until_agent<TPredicate>(
        &mut self,
        agent_id: &str,
        observation_window: ObservationWindow,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        self.until_verdict(observation_window, |snapshot| {
            if snapshot
                .agents
                .iter()
                .any(|registered_agent| registered_agent.id == agent_id)
            {
                Ok(predicate(snapshot))
            } else {
                Err(ClusterHarnessError::AgentLeftThePool {
                    agent_id: agent_id.to_owned(),
                })
            }
        })
        .await
    }

    pub async fn wait_for_agent_ready(
        &mut self,
        agent_name: &str,
        expected_slot_count: i32,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.until_verdict(ObservationWindow::model_load(), |snapshot| {
            match snapshot
                .agents
                .iter()
                .find(|registered_agent| registered_agent.name.as_deref() == Some(agent_name))
            {
                Some(registered_agent) if !registered_agent.issues.is_empty() => {
                    Err(ClusterHarnessError::AgentReportedIssues {
                        agent_name: agent_name.to_owned(),
                        issues: registered_agent.issues.clone(),
                    })
                }
                Some(registered_agent) => Ok(registered_agent.slots_total == expected_slot_count),
                None => Ok(false),
            }
        })
        .await
    }
}

impl SnapshotsWatcher<BufferedRequestManagerSnapshot> {
    #[must_use]
    pub fn of_buffered_requests(stream: BufferedRequestsStream) -> Self {
        Self {
            snapshots_stream: SnapshotsStream::BufferedRequests,
            stream,
        }
    }
}
