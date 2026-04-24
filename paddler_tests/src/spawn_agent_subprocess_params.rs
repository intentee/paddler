use std::net::SocketAddr;

pub struct SpawnAgentSubprocessParams {
    pub management_addr: SocketAddr,
    pub name: Option<String>,
    pub slots: i32,
}
