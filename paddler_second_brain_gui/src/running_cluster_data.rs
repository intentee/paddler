use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;

pub struct RunningClusterData {
    pub agent_snapshots: Vec<AgentControllerSnapshot>,
    pub cluster_address: String,
    pub stopping: bool,
}
