use std::pin::Pin;

use futures_util::FutureExt as _;
use futures_util::Stream;
use futures_util::StreamExt as _;

use paddler_client::agents_stream::AgentsStream;
use paddler_client::buffered_requests_stream::BufferedRequestsStream;
use paddler_client::error::Result as ClientResult;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::snapshots_stream::SnapshotsStream;

type SnapshotItems<TSnapshot> = Pin<Box<dyn Stream<Item = ClientResult<TSnapshot>> + Send>>;

async fn next_newest_snapshot<TSnapshot>(
    snapshots_stream: SnapshotsStream,
    stream: &mut SnapshotItems<TSnapshot>,
) -> Result<TSnapshot, ClusterHarnessError> {
    let mut newest_snapshot = stream
        .next()
        .await
        .ok_or(ClusterHarnessError::SnapshotsStreamClosed { snapshots_stream })?;

    while let Some(Some(buffered_snapshot)) = stream.next().now_or_never() {
        newest_snapshot = buffered_snapshot;
    }

    newest_snapshot.map_err(|source| ClusterHarnessError::SnapshotsStreamFailed {
        snapshots_stream,
        source,
    })
}

pub struct SnapshotsWatcher<TSnapshot> {
    newest_snapshot: TSnapshot,
    snapshots_stream: SnapshotsStream,
    stream: SnapshotItems<TSnapshot>,
}

impl<TSnapshot: Clone> SnapshotsWatcher<TSnapshot> {
    async fn connect(
        snapshots_stream: SnapshotsStream,
        mut stream: SnapshotItems<TSnapshot>,
    ) -> Result<Self, ClusterHarnessError> {
        let newest_snapshot = next_newest_snapshot(snapshots_stream, &mut stream).await?;

        Ok(Self {
            newest_snapshot,
            snapshots_stream,
            stream,
        })
    }

    pub async fn until<TPredicate>(
        &mut self,
        mut predicate: TPredicate,
    ) -> Result<TSnapshot, ClusterHarnessError>
    where
        TPredicate: FnMut(&TSnapshot) -> bool,
    {
        self.until_verdict(|snapshot| Ok(predicate(snapshot))).await
    }

    async fn until_verdict<TVerdict>(
        &mut self,
        mut verdict: TVerdict,
    ) -> Result<TSnapshot, ClusterHarnessError>
    where
        TVerdict: FnMut(&TSnapshot) -> Result<bool, ClusterHarnessError>,
    {
        loop {
            if verdict(&self.newest_snapshot)? {
                return Ok(self.newest_snapshot.clone());
            }

            self.newest_snapshot =
                next_newest_snapshot(self.snapshots_stream, &mut self.stream).await?;
        }
    }
}

impl SnapshotsWatcher<AgentControllerPoolSnapshot> {
    pub async fn of_agents(stream: AgentsStream) -> Result<Self, ClusterHarnessError> {
        Self::connect(SnapshotsStream::Agents, stream).await
    }

    pub async fn until_agent<TPredicate>(
        &mut self,
        agent_id: &str,
        mut predicate: TPredicate,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError>
    where
        TPredicate: FnMut(&AgentControllerPoolSnapshot) -> bool,
    {
        self.until_verdict(|snapshot| {
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
        expected_slot_count: u16,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.until_verdict(|snapshot| {
            match snapshot
                .agents
                .iter()
                .find(|registered_agent| registered_agent.name.as_deref() == Some(agent_name))
            {
                Some(registered_agent) if !registered_agent.status.issues.is_empty() => {
                    Err(ClusterHarnessError::AgentReportedIssues {
                        agent_name: agent_name.to_owned(),
                        issues: registered_agent.status.issues.clone(),
                    })
                }
                Some(registered_agent) => {
                    Ok(registered_agent.status.slots_total == u64::from(expected_slot_count))
                }
                None => Ok(false),
            }
        })
        .await
    }
}

impl SnapshotsWatcher<BufferedRequestManagerSnapshot> {
    pub async fn of_buffered_requests(
        stream: BufferedRequestsStream,
    ) -> Result<Self, ClusterHarnessError> {
        Self::connect(SnapshotsStream::BufferedRequests, stream).await
    }
}

#[cfg(test)]
mod tests {
    use futures_util::stream::iter;

    use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

    use super::SnapshotsWatcher;

    fn snapshot(buffered_requests_current: u64) -> BufferedRequestManagerSnapshot {
        BufferedRequestManagerSnapshot {
            buffered_requests_current,
        }
    }

    #[tokio::test]
    async fn a_wait_is_satisfied_by_the_snapshot_an_earlier_wait_already_received() {
        let mut watcher = SnapshotsWatcher::of_buffered_requests(Box::pin(iter(vec![
            Ok(snapshot(1)),
            Ok(snapshot(2)),
        ])))
        .await
        .expect("the stream delivers its first snapshots");

        watcher
            .until(|received| received.buffered_requests_current >= 1)
            .await
            .expect("the first snapshot satisfies the first wait");

        let satisfied_by = watcher
            .until(|received| received.buffered_requests_current == 2)
            .await
            .expect("the newest snapshot already received satisfies the second wait");

        assert_eq!(satisfied_by.buffered_requests_current, 2);
    }
}
