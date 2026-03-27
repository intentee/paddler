use std::net::SocketAddr;
use std::net::TcpListener;

use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;

fn is_port_in_use(address: &SocketAddr) -> bool {
    TcpListener::bind(address).is_err()
}

#[derive(Debug, Clone)]
pub enum Message {
    SetClusterAddress(String),
    SetInferenceAddress(String),
    SelectModel(ModelPreset),
    Confirm,
    Cancel,
}

#[expect(
    clippy::large_enum_variant,
    reason = "ephemeral value, immediately consumed"
)]
pub enum Action {
    None,
    Cancel,
    StartCluster {
        management_addr: SocketAddr,
        inference_addr: SocketAddr,
        desired_state: BalancerDesiredState,
    },
}

impl StartClusterConfigData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SelectModel(preset) => {
                self.selected_model = Some(preset);
                self.model_error = None;

                Action::None
            }
            Message::SetClusterAddress(address) => {
                self.cluster_address = address;
                self.cluster_address_error = None;

                Action::None
            }
            Message::SetInferenceAddress(address) => {
                self.inference_address = address;
                self.inference_address_error = None;

                Action::None
            }
            Message::Confirm => self.validate_and_confirm(),
            Message::Cancel => Action::Cancel,
        }
    }

    fn validate_and_confirm(&mut self) -> Action {
        self.cluster_address_error = None;
        self.inference_address_error = None;
        self.model_error = None;

        if self.selected_model.is_none() {
            self.model_error = Some("Please select a model.".to_owned());
        }

        let management_addr = if self.cluster_address.is_empty() {
            self.cluster_address_error = Some("Cluster address is required.".to_owned());
            None
        } else if let Ok(addr) = self.cluster_address.parse::<SocketAddr>() {
            Some(addr)
        } else {
            self.cluster_address_error =
                Some("Invalid address, expected format: IP:port".to_owned());
            None
        };

        let inference_addr = if self.inference_address.is_empty() {
            self.inference_address_error = Some("Inference address is required.".to_owned());
            None
        } else if let Ok(addr) = self.inference_address.parse::<SocketAddr>() {
            Some(addr)
        } else {
            self.inference_address_error =
                Some("Invalid address, expected format: IP:port".to_owned());
            None
        };

        let management_addr = match management_addr {
            Some(addr) if is_port_in_use(&addr) => {
                self.cluster_address_error =
                    Some(format!("Port {} is already in use", addr.port()));
                None
            }
            other => other,
        };

        let inference_addr = match inference_addr {
            Some(addr) if is_port_in_use(&addr) => {
                self.inference_address_error =
                    Some(format!("Port {} is already in use", addr.port()));
                None
            }
            other => other,
        };

        if self.model_error.is_some()
            || self.cluster_address_error.is_some()
            || self.inference_address_error.is_some()
        {
            return Action::None;
        }

        let (Some(management_addr), Some(inference_addr)) = (management_addr, inference_addr)
        else {
            return Action::None;
        };

        let desired_state = self
            .selected_model
            .as_ref()
            .map(ModelPreset::to_balancer_desired_state)
            .unwrap_or_default();

        self.starting = true;

        Action::StartCluster {
            management_addr,
            inference_addr,
            desired_state,
        }
    }
}
