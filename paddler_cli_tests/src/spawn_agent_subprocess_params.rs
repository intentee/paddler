use std::net::SocketAddr;

pub struct SpawnAgentSubprocessParams {
    pub binary_path: String,
    pub management_addr: SocketAddr,
    pub name: String,
    pub slots: i32,
}
