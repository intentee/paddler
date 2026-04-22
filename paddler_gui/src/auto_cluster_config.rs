use std::net::SocketAddr;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub struct AutoClusterConfig {
    pub inference_addr: SocketAddr,
    pub management_addr: SocketAddr,
}

static AUTO_CLUSTER_CONFIG: OnceLock<AutoClusterConfig> = OnceLock::new();

pub fn install_auto_cluster_config(config: AutoClusterConfig) {
    let _ = AUTO_CLUSTER_CONFIG.set(config);
}

#[must_use]
pub fn get_auto_cluster_config() -> Option<AutoClusterConfig> {
    AUTO_CLUSTER_CONFIG.get().copied()
}
