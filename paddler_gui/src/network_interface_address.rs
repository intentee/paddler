use std::net::IpAddr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkInterfaceAddress {
    pub interface_name: String,
    pub ip_address: IpAddr,
}
