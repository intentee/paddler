#[derive(Debug, Clone)]
pub enum Message {
    StartCluster,
    Cancel,
    SelectModel(String),
    ToggleRunAgentLocally(bool),
    Confirm,
    ClusterStarted,
    ClusterFailed(String),
    Stop,
    ClusterStopped,
}
