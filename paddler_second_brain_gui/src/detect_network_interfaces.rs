use std::net::IpAddr;

use crate::network_interface_address::NetworkInterfaceAddress;

pub fn detect_network_interfaces() -> Vec<NetworkInterfaceAddress> {
    let interfaces = match if_addrs::get_if_addrs() {
        Ok(interfaces) => interfaces,
        Err(error) => {
            log::error!("Failed to detect network interfaces: {error}");

            return Vec::new();
        }
    };

    interfaces
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter(|interface| match interface.ip() {
            IpAddr::V4(ipv4) => ipv4.is_private(),
            IpAddr::V6(_) => false,
        })
        .map(|interface| {
            let ip_address = interface.ip();

            NetworkInterfaceAddress {
                interface_name: interface.name,
                ip_address,
            }
        })
        .collect()
}
