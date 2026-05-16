use std::net::SocketAddr;

pub struct BindAddresses {
    pub inference_addr: SocketAddr,
    pub management_addr: SocketAddr,
}
