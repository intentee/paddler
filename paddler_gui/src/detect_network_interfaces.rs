use crate::network_interface_address::NetworkInterfaceAddress;

#[must_use]
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
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;

    use super::detect_network_interfaces;

    #[test]
    fn detected_addresses_are_ipv4_and_not_loopback() -> Result<()> {
        for address in detect_network_interfaces() {
            assert!(
                address.ip_address.is_ipv4(),
                "expected only ipv4 addresses, got {}",
                address.ip_address
            );
            assert!(
                !address.ip_address.is_loopback(),
                "expected loopback to be filtered, got {}",
                address.ip_address
            );
        }

        Ok(())
    }
}
