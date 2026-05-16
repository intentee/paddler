use std::net::SocketAddr;
use std::net::TcpListener;

pub struct BoundPort {
    pub socket_addr: SocketAddr,
    pub listener: TcpListener,
}
