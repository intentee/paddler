use std::net::SocketAddr;

#[derive(Clone, Copy)]
pub struct BindAddresses {
    pub inference_addr: SocketAddr,
    pub management_addr: SocketAddr,
}
