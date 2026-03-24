use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum Message {
    AgentFailed(String),
    AgentSnapshotsUpdated(Vec<AgentControllerSnapshot>),
    AgentStatusUpdated(SlotAggregatedStatusSnapshot),
    AgentStopped,
    Cancel,
    ClusterFailed(String),
    ClusterStarted,
    ClusterStopped,
    Confirm,
    Connect,
    CopyToClipboard(String),
    Disconnect,
    JoinCluster,
    SelectModel(ModelPreset),
    SetAgentName(String),
    SetBalancerAddress(String),
    SetClusterAddress(String),
    SetInferenceAddress(String),
    SetSlotsCount(String),
    StartCluster,
    Stop,
}
