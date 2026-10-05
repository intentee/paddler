use std::net::SocketAddr;

#[derive(Clone)]
pub struct ResolvedSocketAddr {
    pub input_addr: String,
    pub socket_addr: SocketAddr,
}

impl From<SocketAddr> for ResolvedSocketAddr {
    fn from(socket_addr: SocketAddr) -> Self {
        Self {
            input_addr: socket_addr.to_string(),
            socket_addr,
        }
    }
}
