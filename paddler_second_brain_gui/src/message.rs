use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum Message {
    AgentFailed(String),
    AgentStopped,
    Connect,
    SetAgentName(String),
    Disconnect,
    JoinCluster,
    RefreshAgentStatus,
    SetClusterAddress(String),
    SetSlotsCount(String),
    StartCluster,
    Cancel,
    CopyToClipboard(String),
    SetBalancerAddress(String),
    SetInferenceAddress(String),
    SelectModel(ModelPreset),
    Confirm,
    ClusterStarted,
    ClusterFailed(String),
    Stop,
    ClusterStopped,
    RefreshAgentCount,
}
