use anyhow::Context as _;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use crate::connection_bound_stream::ConnectionBoundStream;
use crate::connection_keep_alive::ConnectionKeepAlive;
use crate::snapshots_watcher::SnapshotsWatcher;

pub struct ClusterSnapshotsWatchers {
    pub agents_watcher: SnapshotsWatcher<AgentControllerPoolSnapshot>,
    pub buffered_requests_watcher: SnapshotsWatcher<BufferedRequestManagerSnapshot>,
}

impl ClusterSnapshotsWatchers {
    /// # Errors
    ///
    /// Fails when the agents or buffered requests stream cannot be opened, and returns [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when such a stream ends or fails before its first snapshot.
    pub async fn open(
        client_management: &ClientManagement,
        cancellation_token: CancellationToken,
        connection_keep_alive: ConnectionKeepAlive,
    ) -> Result<Self> {
        let agents_stream = client_management
            .get_agents_stream(cancellation_token.clone())
            .await
            .context("failed to open the agents stream")?;
        let agents_watcher = SnapshotsWatcher::of_agents(Box::pin(ConnectionBoundStream {
            connection_keep_alive: connection_keep_alive.clone(),
            stream: agents_stream,
        }))
        .await?;
        let buffered_requests_stream = client_management
            .get_buffered_requests_stream(cancellation_token)
            .await
            .context("failed to open the buffered requests stream")?;
        let buffered_requests_watcher =
            SnapshotsWatcher::of_buffered_requests(Box::pin(ConnectionBoundStream {
                connection_keep_alive,
                stream: buffered_requests_stream,
            }))
            .await?;

        Ok(Self {
            agents_watcher,
            buffered_requests_watcher,
        })
    }
}
