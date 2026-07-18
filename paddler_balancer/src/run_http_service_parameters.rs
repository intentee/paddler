use std::net::SocketAddr;

pub struct RunHttpServiceParameters<TAppFactory> {
    pub app_factory: TAppFactory,
    pub bind_addr: SocketAddr,
    pub service_name: &'static str,
    pub worker_count: usize,
}
