use tokio_util::sync::CancellationToken;

use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

pub struct AgentControllerParams {
    pub connection_close: CancellationToken,
    pub id: String,
    pub name: Option<String>,
    pub reported_status: SlotAggregatedStatusSnapshot,
}
