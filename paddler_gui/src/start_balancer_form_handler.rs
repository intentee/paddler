use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::model_preset::ModelPreset;
use crate::start_balancer_form_data::StartBalancerFormData;

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
        .map_err(|error| format!("Invalid address ({error}), expected format: IP:port"))?;

    match check_port(&addr) {
        PortCheck::Available => Ok(Some(addr)),
        PortCheck::InUse => Err(format!("Port {} is already in use", addr.port())),
        PortCheck::BindFailed(error) => Err(format!("Cannot bind to {addr}: {error}")),
    }
}

fn validate_required_address(raw: &str) -> Result<SocketAddr, String> {
    validate_optional_address(raw)?.ok_or_else(|| "Address is required.".to_owned())
}

#[expect(
    clippy::large_enum_variant,
    reason = "ephemeral value, immediately consumed"
)]
#[derive(Debug, Clone)]
pub enum Message {
    SetBalancerAddress(String),
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
    StartBalancer {
        management_addr: SocketAddr,
        inference_addr: SocketAddr,
        web_admin_panel_addr: Option<SocketAddr>,
        desired_state: BalancerDesiredState,
    },
}

impl StartBalancerFormData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SelectModel(preset) => {
                self.selected_model = Some(preset);
                self.model_error = None;

                Action::None
            }
            Message::SetBalancerAddress(address) => {
                self.balancer_address = address;
                self.balancer_address_error = None;

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
        self.balancer_address_error = None;
        self.inference_address_error = None;
        self.web_admin_panel_address_error = None;
        self.model_error = None;

        if !self.add_model_later && self.selected_model.is_none() {
            self.model_error = Some("Please select a model.".to_owned());
        }

        let management_addr = match validate_required_address(&self.balancer_address) {
            Ok(addr) => Some(addr),
            Err(message) => {
                self.balancer_address_error = Some(message);
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
            || self.balancer_address_error.is_some()
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

        Action::StartBalancer {
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

    use paddler_types::agent_desired_model::AgentDesiredModel;

    use super::Action;
    use super::Message;
    use super::StartBalancerFormData;
    use crate::model_preset::ModelPreset;

    fn empty_form() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: String::new(),
            balancer_address_error: None,
            inference_address: String::new(),
            inference_address_error: None,
            model_error: None,
            selected_model: None,
            starting: false,
            web_admin_panel_address: String::new(),
            web_admin_panel_address_error: None,
            web_admin_panel_address_placeholder: String::new(),
        }
    }

    fn first_preset() -> Result<ModelPreset> {
        ModelPreset::available_presets()
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("at least one preset must exist"))
    }

    fn loopback_socket_with_free_port() -> Result<SocketAddr> {
        let listener = TcpListener::bind(LOOPBACK_ANY_PORT)?;
        let addr = listener.local_addr()?;
        drop(listener);
        Ok(addr)
    }

    #[test]
    fn selecting_a_model_clears_a_previously_set_model_error() -> Result<()> {
        let mut data = empty_form();
        data.model_error = Some("stale".to_owned());

        let _ = data.update(Message::SelectModel(first_preset()?));

        if data.model_error.is_some() {
            bail!("expected model_error to be cleared after SelectModel");
        }

        Ok(())
    }

    #[test]
    fn setting_balancer_address_clears_a_previously_set_balancer_address_error() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address_error = Some("stale".to_owned());

        let _ = data.update(Message::SetBalancerAddress("127.0.0.1:8060".to_owned()));

        if data.balancer_address_error.is_some() {
            bail!("expected balancer_address_error to be cleared");
        }
        if data.balancer_address != "127.0.0.1:8060" {
            bail!("expected balancer_address to be updated");
        }
        Ok(())
    }

    #[test]
    fn setting_inference_address_clears_a_previously_set_inference_address_error() -> Result<()> {
        let mut data = empty_form();
        data.inference_address_error = Some("stale".to_owned());

        let _ = data.update(Message::SetInferenceAddress("127.0.0.1:8061".to_owned()));

        if data.inference_address_error.is_some() {
            bail!("expected inference_address_error to be cleared");
        }
        Ok(())
    }

    #[test]
    fn setting_web_admin_panel_address_clears_a_previously_set_web_admin_panel_address_error()
    -> Result<()> {
        let mut data = empty_form();
        data.web_admin_panel_address_error = Some("stale".to_owned());

        let _ = data.update(Message::SetWebAdminPanelAddress("127.0.0.1:8062".to_owned()));

        if data.web_admin_panel_address_error.is_some() {
            bail!("expected web_admin_panel_address_error to be cleared");
        }
        Ok(())
    }

    #[test]
    fn toggling_add_model_later_on_clears_a_previously_set_model_error() -> Result<()> {
        let mut data = empty_form();
        data.model_error = Some("stale".to_owned());

        let _ = data.update(Message::ToggleAddModelLater(true));

        if !data.add_model_later {
            bail!("expected add_model_later to flip to true");
        }
        if data.model_error.is_some() {
            bail!("expected model_error to be cleared when toggling on");
        }
        Ok(())
    }

    #[test]
    fn toggling_add_model_later_off_leaves_a_previously_set_model_error_in_place() -> Result<()> {
        let mut data = empty_form();
        data.add_model_later = true;
        data.model_error = Some("preserved".to_owned());

        let _ = data.update(Message::ToggleAddModelLater(false));

        if data.add_model_later {
            bail!("expected add_model_later to flip to false");
        }
        if data.model_error.as_deref() != Some("preserved") {
            bail!("expected model_error to be preserved when toggling off");
        }
        Ok(())
    }

    #[test]
    fn cancel_message_returns_cancel_action() -> Result<()> {
        let mut data = empty_form();

        match data.update(Message::Cancel) {
            Action::Cancel => Ok(()),
            _ => bail!("expected Action::Cancel"),
        }
    }

    #[test]
    fn confirming_without_a_selected_model_records_a_model_required_error() -> Result<()> {
        let mut data = empty_form();
        let management_addr = loopback_socket_with_free_port()?;
        let inference_addr = loopback_socket_with_free_port()?;
        data.balancer_address = management_addr.to_string();
        data.inference_address = inference_addr.to_string();

        match data.update(Message::Confirm) {
            Action::None => {}
            _ => bail!("expected Action::None when validation fails"),
        }

        if data.model_error.is_none() {
            bail!("expected model_error to be set when no model is selected");
        }

        Ok(())
    }

    #[test]
    fn confirming_with_an_in_use_balancer_port_records_an_address_error() -> Result<()> {
        let bound_listener = TcpListener::bind(LOOPBACK_ANY_PORT)?;
        let bound_address = bound_listener.local_addr()?;

        let mut data = empty_form();
        data.balancer_address = bound_address.to_string();
        data.inference_address = loopback_socket_with_free_port()?.to_string();
        data.selected_model = Some(first_preset()?);

        let _ = data.update(Message::Confirm);

        match data.balancer_address_error.as_deref() {
            Some(message) if message.contains("already in use") => Ok(()),
            other => bail!("expected in-use error, got {other:?}"),
        }
    }

    #[test]
    fn confirming_with_an_unparseable_inference_address_records_an_inference_error() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = loopback_socket_with_free_port()?.to_string();
        data.inference_address = "not-a-socket-addr".to_owned();
        data.selected_model = Some(first_preset()?);

        let _ = data.update(Message::Confirm);

        if data.inference_address_error.is_none() {
            bail!("expected inference_address_error to be set for unparseable input");
        }

        Ok(())
    }

    #[test]
    fn confirming_with_an_unparseable_web_admin_panel_address_records_a_web_admin_error()
    -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = loopback_socket_with_free_port()?.to_string();
        data.inference_address = loopback_socket_with_free_port()?.to_string();
        data.web_admin_panel_address = "not-a-socket-addr".to_owned();
        data.selected_model = Some(first_preset()?);

        let _ = data.update(Message::Confirm);

        if data.web_admin_panel_address_error.is_none() {
            bail!("expected web_admin_panel_address_error to be set for unparseable input");
        }

        Ok(())
    }

    #[test]
    fn confirming_with_valid_input_and_selected_model_returns_start_balancer_action() -> Result<()>
    {
        let mut data = empty_form();
        data.balancer_address = loopback_socket_with_free_port()?.to_string();
        data.inference_address = loopback_socket_with_free_port()?.to_string();
        data.selected_model = Some(first_preset()?);

        match data.update(Message::Confirm) {
            Action::StartBalancer { desired_state, .. } => {
                if !data.starting {
                    bail!("expected starting flag to be set after StartBalancer action");
                }
                match desired_state.model {
                    AgentDesiredModel::HuggingFace(_) => Ok(()),
                    other => bail!("expected HuggingFace model from preset, got {other:?}"),
                }
            }
            _ => bail!("expected Action::StartBalancer for valid input with preset"),
        }
    }

    #[test]
    fn confirming_with_add_model_later_returns_start_balancer_with_default_desired_state()
    -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = loopback_socket_with_free_port()?.to_string();
        data.inference_address = loopback_socket_with_free_port()?.to_string();
        data.add_model_later = true;

        match data.update(Message::Confirm) {
            Action::StartBalancer { desired_state, .. } => {
                match desired_state.model {
                    AgentDesiredModel::None => Ok(()),
                    other => bail!("expected default (None) model, got {other:?}"),
                }
            }
            _ => bail!("expected Action::StartBalancer for valid input with add_model_later"),
        }
    }
}
