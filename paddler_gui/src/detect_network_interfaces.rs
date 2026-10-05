use if_addrs::get_if_addrs;
use log::error;

use crate::network_interface_address::NetworkInterfaceAddress;

#[must_use]
pub fn detect_network_interfaces() -> Vec<NetworkInterfaceAddress> {
    let interfaces = match get_if_addrs() {
        Ok(interfaces) => interfaces,
        Err(error) => {
            error!("Failed to detect network interfaces: {error}");

            return Vec::new();
        }
    };

    interfaces
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter(|interface| interface.ip().is_ipv4())
        .map(|interface| {
            let ip_address = interface.ip();

            NetworkInterfaceAddress {
                interface_name: interface.name,
                ip_address,
            }
        })
        .collect()
}
