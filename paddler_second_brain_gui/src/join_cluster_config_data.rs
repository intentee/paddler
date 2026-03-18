#[derive(Default)]
pub struct JoinClusterConfigData {
    pub agent_name: String,
    pub cluster_address: String,
    pub error: Option<String>,
    pub slots_count: String,
}
