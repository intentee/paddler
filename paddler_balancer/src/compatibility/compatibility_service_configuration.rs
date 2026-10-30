use crate::resolved_socket_addr::ResolvedSocketAddr;

#[derive(Clone)]
pub struct CompatibilityServiceConfiguration {
    pub addr: ResolvedSocketAddr,
}
