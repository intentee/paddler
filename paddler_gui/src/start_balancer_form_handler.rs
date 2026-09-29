use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

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

    fn desired_state(&self) -> Result<BalancerDesiredState, String> {
        if self.add_model_later {
            return Ok(BalancerDesiredState::default());
        }

        self.selected_model
            .as_ref()
            .map(ModelPreset::to_balancer_desired_state)
            .ok_or_else(|| "Please select a model.".to_owned())
    }

    fn validate_and_confirm(&mut self) -> Action {
        let desired_state = self.desired_state();
        let management_addr = validate_required_address(&self.balancer_address);
        let inference_addr = validate_required_address(&self.inference_address);
        let web_admin_panel_addr = validate_optional_address(&self.web_admin_panel_address);

        self.model_error = desired_state.as_ref().err().cloned();
        self.balancer_address_error = management_addr.as_ref().err().cloned();
        self.inference_address_error = inference_addr.as_ref().err().cloned();
        self.web_admin_panel_address_error = web_admin_panel_addr.as_ref().err().cloned();

        let (Ok(desired_state), Ok(management_addr), Ok(inference_addr), Ok(web_admin_panel_addr)) = (
            desired_state,
            management_addr,
            inference_addr,
            web_admin_panel_addr,
        ) else {
            return Action::None;
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
    use std::net::TcpListener;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::Action;
    use super::Message;
    use crate::model_preset::ModelPreset;
    use crate::start_balancer_form_data::StartBalancerFormData;

    const LOOPBACK_ANY_PORT: &str = "127.0.0.1:0";
    const UNASSIGNED_TEST_NET_ADDRESS: &str = "192.0.2.1:0";

    fn form_on_free_ports() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: LOOPBACK_ANY_PORT.to_owned(),
            balancer_address_error: None,
            inference_address: LOOPBACK_ANY_PORT.to_owned(),
            inference_address_error: None,
            model_error: None,
            selected_model: None,
            starting: false,
            web_admin_panel_address: String::new(),
            web_admin_panel_address_error: None,
            web_admin_panel_address_placeholder: String::new(),
        }
    }

    fn multimodal_preset() -> ModelPreset {
        ModelPreset::available_presets()
            .into_iter()
            .find(|preset| preset.multimodal_projection.is_some())
            .expect("a multimodal preset must be available")
    }

    #[test]
    fn requires_a_model_unless_it_is_added_later() {
        let mut form = form_on_free_ports();

        assert!(matches!(form.update(Message::Confirm), Action::None));
        assert_eq!(form.model_error.as_deref(), Some("Please select a model."));
        assert!(!form.starting);
    }

    #[test]
    fn starts_without_a_model_when_it_is_added_later() {
        let mut form = form_on_free_ports();

        form.update(Message::ToggleAddModelLater(true));

        assert!(matches!(
            form.update(Message::Confirm),
            Action::StartBalancer {
                desired_state,
                web_admin_panel_addr: None,
                ..
            } if desired_state == BalancerDesiredState::default()
        ));
        assert!(form.starting);
    }

    #[test]
    fn starts_with_the_selected_model_and_its_multimodal_projection() {
        let mut form = form_on_free_ports();
        let preset = multimodal_preset();
        let expected_model = AgentDesiredModel::HuggingFace(preset.model.clone());
        let expected_multimodal_projection = AgentDesiredModel::HuggingFace(
            preset
                .multimodal_projection
                .clone()
                .expect("the multimodal preset must carry a projection"),
        );

        form.update(Message::SelectModel(preset));

        assert!(matches!(
            form.update(Message::Confirm),
            Action::StartBalancer { desired_state, .. }
                if desired_state.model == expected_model
                    && desired_state.multimodal_projection == expected_multimodal_projection
        ));
    }

    #[test]
    fn reports_every_invalid_address_at_once() {
        let mut form = form_on_free_ports();

        form.update(Message::ToggleAddModelLater(true));
        form.update(Message::SetBalancerAddress(String::new()));
        form.update(Message::SetInferenceAddress("not an address".to_owned()));
        form.update(Message::SetWebAdminPanelAddress("127.0.0.1".to_owned()));

        assert!(matches!(form.update(Message::Confirm), Action::None));
        assert_eq!(
            form.balancer_address_error.as_deref(),
            Some("Address is required.")
        );
        assert_eq!(
            form.inference_address_error.as_deref(),
            Some("Invalid address (invalid socket address syntax), expected format: IP:port")
        );
        assert_eq!(
            form.web_admin_panel_address_error.as_deref(),
            Some("Invalid address (invalid socket address syntax), expected format: IP:port")
        );
    }

    #[test]
    fn reports_an_address_another_listener_holds_as_in_use() {
        let competing_listener =
            TcpListener::bind(LOOPBACK_ANY_PORT).expect("the competing listener must bind");
        let taken_address = competing_listener
            .local_addr()
            .expect("the competing listener must report its address");
        let mut form = form_on_free_ports();

        form.update(Message::ToggleAddModelLater(true));
        form.update(Message::SetInferenceAddress(taken_address.to_string()));

        assert!(matches!(form.update(Message::Confirm), Action::None));
        assert_eq!(
            form.inference_address_error,
            Some(format!("Port {} is already in use", taken_address.port()))
        );
    }

    #[test]
    fn reports_why_an_address_cannot_be_bound() {
        let bind_error = TcpListener::bind(UNASSIGNED_TEST_NET_ADDRESS)
            .expect_err("an address assigned to no interface must not bind");
        let mut form = form_on_free_ports();

        form.update(Message::ToggleAddModelLater(true));
        form.update(Message::SetWebAdminPanelAddress(
            UNASSIGNED_TEST_NET_ADDRESS.to_owned(),
        ));

        assert!(matches!(form.update(Message::Confirm), Action::None));
        assert_eq!(
            form.web_admin_panel_address_error,
            Some(format!(
                "Cannot bind to {UNASSIGNED_TEST_NET_ADDRESS}: {bind_error}"
            ))
        );
    }
}
