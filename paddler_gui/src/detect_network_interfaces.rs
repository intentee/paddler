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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;

    use super::detect_network_interfaces;

    #[test]
    fn detected_addresses_are_ipv4_and_not_loopback() -> Result<()> {
        for address in detect_network_interfaces() {
            if !address.ip_address.is_ipv4() {
                bail!("expected only ipv4 addresses, got {}", address.ip_address);
            }

            if address.ip_address.is_loopback() {
                bail!("expected loopback to be filtered, got {}", address.ip_address);
            }
        }

        Ok(())
    }
}
