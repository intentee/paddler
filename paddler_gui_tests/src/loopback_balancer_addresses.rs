use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use paddler_balancer::balancer_addresses::BalancerAddresses;

pub const LOOPBACK_BALANCER_ADDRESSES: BalancerAddresses = BalancerAddresses {
    compat_openai: None,
    inference: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8061),
    management: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8060),
    web_admin_panel: None,
};
