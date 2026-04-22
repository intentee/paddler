#[derive(Default)]
pub struct JoinClusterConfigData {
    pub agent_name: String,
    pub cluster_address: String,
    pub cluster_address_error: Option<String>,
    pub slots_count: String,
    pub slots_error: Option<String>,
}
