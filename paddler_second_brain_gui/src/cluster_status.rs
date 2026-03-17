pub enum ClusterStatus {
    Stopped,
    Running,
    Failed(String),
}
