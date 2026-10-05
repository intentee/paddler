use std::net::SocketAddr;

#[derive(Clone)]
pub struct Configuration {
    pub addr: SocketAddr,
}
