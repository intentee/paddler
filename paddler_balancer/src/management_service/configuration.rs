use std::net::SocketAddr;

#[derive(Clone)]
pub struct Configuration {
    pub addr: SocketAddr,
    pub cors_allowed_hosts: Vec<String>,
}
