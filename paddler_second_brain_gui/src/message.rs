use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum Message {
    StartCluster,
    Cancel,
    SelectModel(ModelPreset),
    ToggleRunAgentLocally(bool),
    Confirm,
    ClusterStarted,
    ClusterFailed(String),
    Stop,
    ClusterStopped,
    RefreshNetworkInterfaces,
}
