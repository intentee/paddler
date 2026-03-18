#[derive(Default)]
pub struct JoinClusterConfigData {
    pub cluster_address: String,
    pub error: Option<String>,
    pub slots_count: String,
}
