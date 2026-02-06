use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAgentStatusParams {
    pub slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
}
