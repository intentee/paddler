use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum Message {
    AgentFailed(String),
    AgentStopped,
    Connect,
    Disconnect,
    JoinCluster,
    RefreshAgentStatus,
    SetClusterAddress(String),
    SetSlotsCount(String),
    StartCluster,
    Cancel,
    SelectModel(ModelPreset),
    ToggleRunAgentLocally(bool),
    Confirm,
    ClusterStarted,
    ClusterFailed(String),
    Stop,
    ClusterStopped,
    RefreshAgentCount,
    RefreshNetworkInterfaces,
}
