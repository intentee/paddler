use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;

enum PortCheck {
    Available,
    InUse,
    BindFailed(io::Error),
}

fn check_port(address: &SocketAddr) -> PortCheck {
    match TcpListener::bind(address) {
        Ok(_) => PortCheck::Available,
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => PortCheck::InUse,
        Err(error) => PortCheck::BindFailed(error),
    }
}

fn validate_optional_address(raw: &str) -> Result<Option<SocketAddr>, String> {
    if raw.is_empty() {
        return Ok(None);
    }

    let addr = raw
        .parse::<SocketAddr>()
        .map_err(|_| "Invalid address, expected format: IP:port".to_owned())?;

    match check_port(&addr) {
        PortCheck::Available => Ok(Some(addr)),
        PortCheck::InUse => Err(format!("Port {} is already in use", addr.port())),
        PortCheck::BindFailed(error) => Err(format!("Cannot bind to {addr}: {error}")),
    }
}

fn validate_required_address(raw: &str) -> Result<SocketAddr, String> {
    validate_optional_address(raw)?.ok_or_else(|| "Address is required.".to_owned())
}

#[derive(Debug, Clone)]
pub enum Message {
    SetClusterAddress(String),
    SetInferenceAddress(String),
    SetWebAdminPanelAddress(String),
    SelectModel(ModelPreset),
    ToggleAddModelLater(bool),
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
        web_admin_panel_addr: Option<SocketAddr>,
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
            Message::SetWebAdminPanelAddress(address) => {
                self.web_admin_panel_address = address;
                self.web_admin_panel_address_error = None;

                Action::None
            }
            Message::ToggleAddModelLater(add_later) => {
                self.add_model_later = add_later;

                if add_later {
                    self.model_error = None;
                }

                Action::None
            }
            Message::Confirm => self.validate_and_confirm(),
            Message::Cancel => Action::Cancel,
        }
    }

    fn validate_and_confirm(&mut self) -> Action {
        self.cluster_address_error = None;
        self.inference_address_error = None;
        self.web_admin_panel_address_error = None;
        self.model_error = None;

        if !self.add_model_later && self.selected_model.is_none() {
            self.model_error = Some("Please select a model.".to_owned());
        }

        let management_addr = match validate_required_address(&self.cluster_address) {
            Ok(addr) => Some(addr),
            Err(message) => {
                self.cluster_address_error = Some(message);
                None
            }
        };

        let inference_addr = match validate_required_address(&self.inference_address) {
            Ok(addr) => Some(addr),
            Err(message) => {
                self.inference_address_error = Some(message);
                None
            }
        };

        let web_admin_panel_addr = match validate_optional_address(&self.web_admin_panel_address) {
            Ok(addr) => addr,
            Err(message) => {
                self.web_admin_panel_address_error = Some(message);
                None
            }
        };

        if self.model_error.is_some()
            || self.cluster_address_error.is_some()
            || self.inference_address_error.is_some()
            || self.web_admin_panel_address_error.is_some()
        {
            return Action::None;
        }

        let (Some(management_addr), Some(inference_addr)) = (management_addr, inference_addr)
        else {
            return Action::None;
        };

        let desired_state = if self.add_model_later {
            BalancerDesiredState::default()
        } else {
            self.selected_model
                .as_ref()
                .map(ModelPreset::to_balancer_desired_state)
                .unwrap_or_default()
        };

        self.starting = true;

        Action::StartCluster {
            management_addr,
            inference_addr,
            web_admin_panel_addr,
            desired_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::net::TcpListener;

    use anyhow::Result;
    use anyhow::bail;

    use super::PortCheck;
    use super::check_port;
    use super::validate_optional_address;
    use super::validate_required_address;

    const LOOPBACK_ANY_PORT: &str = "127.0.0.1:0";
    const UNASSIGNED_TEST_NET_ADDRESS: &str = "192.0.2.1:0";

    #[test]
    fn reports_in_use_when_port_is_bound() -> Result<()> {
        let listener = TcpListener::bind(LOOPBACK_ANY_PORT)?;
        let bound_address = listener.local_addr()?;

        match check_port(&bound_address) {
            PortCheck::InUse => Ok(()),
            PortCheck::Available => bail!("bound port reported as Available"),
            PortCheck::BindFailed(error) => {
                bail!("bound port reported as BindFailed: {error}")
            }
        }
    }

    #[test]
    fn reports_available_when_port_is_free() -> Result<()> {
        let listener = TcpListener::bind(LOOPBACK_ANY_PORT)?;
        let bound_address = listener.local_addr()?;

        drop(listener);

        match check_port(&bound_address) {
            PortCheck::Available => Ok(()),
            PortCheck::InUse => bail!("free port reported as InUse"),
            PortCheck::BindFailed(error) => {
                bail!("free port reported as BindFailed: {error}")
            }
        }
    }

    #[test]
    fn reports_bind_failed_for_non_addr_in_use_error() -> Result<()> {
        let unassigned_address: SocketAddr = UNASSIGNED_TEST_NET_ADDRESS.parse()?;

        match check_port(&unassigned_address) {
            PortCheck::BindFailed(_) => Ok(()),
            PortCheck::InUse => {
                bail!("non-AddrInUse bind failure reported as InUse")
            }
            PortCheck::Available => {
                bail!("bind should fail against an unassigned address")
            }
        }
    }

    #[test]
    fn required_address_rejects_empty_input() -> Result<()> {
        match validate_required_address("") {
            Err(_) => Ok(()),
            Ok(address) => bail!("empty required input should not parse, got {address}"),
        }
    }

    #[test]
    fn optional_address_treats_empty_as_none() -> Result<()> {
        match validate_optional_address("") {
            Ok(None) => Ok(()),
            Ok(Some(address)) => bail!("empty optional input should not parse to {address}"),
            Err(error) => bail!("empty optional input should not error: {error}"),
        }
    }
}
