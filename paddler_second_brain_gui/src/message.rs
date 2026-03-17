#[derive(Debug, Clone)]
pub enum Message {
    StartCluster,
    StopCluster,
    ClusterStopped,
    ClusterFailed(String),
}
