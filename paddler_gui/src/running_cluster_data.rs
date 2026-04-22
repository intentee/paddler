use crate::running_cluster_snapshot::RunningClusterSnapshot;

pub struct RunningClusterData {
    pub cluster_address: String,
    pub snapshot: RunningClusterSnapshot,
    pub stopping: bool,
}
