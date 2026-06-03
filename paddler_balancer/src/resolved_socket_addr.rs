use std::net::SocketAddr;

#[derive(Clone)]
pub struct ResolvedSocketAddr {
    pub input_addr: String,
    pub socket_addr: SocketAddr,
}
