use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use crate::snapshots_watcher::SnapshotsWatcher;

pub struct ClusterSnapshotsWatchers {
    pub agents_watcher: SnapshotsWatcher<AgentControllerPoolSnapshot>,
    pub buffered_requests_watcher: SnapshotsWatcher<BufferedRequestManagerSnapshot>,
}

impl ClusterSnapshotsWatchers {
    pub async fn open(
        client_management: &ClientManagement,
        cancellation_token: CancellationToken,
    ) -> Result<Self> {
        client_management
            .get_agents_stream(cancellation_token.clone())
            .map(|agents_stream| agents_stream.context("failed to open the agents stream"))
            .and_then(|agents_stream| SnapshotsWatcher::of_agents(agents_stream).err_into())
            .and_then(|agents_watcher| {
                client_management
                    .get_buffered_requests_stream(cancellation_token)
                    .map(|buffered_requests_stream| {
                        buffered_requests_stream
                            .context("failed to open the buffered requests stream")
                    })
                    .and_then(|buffered_requests_stream| {
                        SnapshotsWatcher::of_buffered_requests(buffered_requests_stream).err_into()
                    })
                    .map_ok(|buffered_requests_watcher| Self {
                        agents_watcher,
                        buffered_requests_watcher,
                    })
            })
            .await
    }
}
