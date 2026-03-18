use crate::network_interface_address::NetworkInterfaceAddress;

pub struct RunningClusterData {
    pub agent_count: usize,
    pub network_interfaces: Vec<NetworkInterfaceAddress>,
    pub management_port: u16,
    pub selected_model_name: String,
    pub run_agent_locally: bool,
}
