use std::net::IpAddr;
use std::net::SocketAddr;

use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_balancer_runner::balancer_defaults::BalancerDefaults;
use paddler_balancer_runner::balancer_runner_config::BalancerRunnerConfig;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::address_placeholder::ADDRESS_PLACEHOLDER;
use crate::balancer_launch::BalancerLaunch;
use crate::form_field_error::FormFieldError;
use crate::inference_mode_choice::InferenceModeChoice;
use crate::model_preset::ModelPreset;
use crate::port_check::PortCheck;
use crate::start_balancer_form_action::StartBalancerFormAction;
use crate::start_balancer_form_message::StartBalancerFormMessage;

fn balancer_runner_config(
    desired_state: BalancerDesiredState,
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
    #[cfg_attr(
        not(feature = "web_admin_panel"),
        expect(
            unused_variables,
            reason = "the web admin panel is only served when the feature is enabled"
        )
    )]
    web_admin_panel_addr: Option<SocketAddr>,
) -> BalancerRunnerConfig {
    BalancerRunnerConfig {
        buffered_request_timeout: BalancerDefaults::BUFFERED_REQUEST_TIMEOUT,
        inference_service_configuration: InferenceServiceConfiguration {
            addr: ResolvedSocketAddr::from(inference_addr),
            cors_allowed_hosts: vec![],
            inference_item_timeout: BalancerDefaults::INFERENCE_ITEM_TIMEOUT,
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: ResolvedSocketAddr::from(management_addr),
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: BalancerDefaults::MAX_BUFFERED_REQUESTS,
        openai_service_configuration: None,
        state_database_type: StateDatabaseType::Memory(Box::new(desired_state)),
        statsd_prefix: BalancerDefaults::STATSD_PREFIX.to_owned(),
        statsd_service_configuration: None,
        typesafe_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: web_admin_panel_addr
            .map(|addr| WebAdminPanelServiceConfiguration { addr }),
    }
}

fn validate_optional_address(raw: &str) -> Result<Option<SocketAddr>, FormFieldError> {
    if raw.is_empty() {
        return Ok(None);
    }

    let addr = raw
        .parse::<SocketAddr>()
        .map_err(|source| FormFieldError::AddressUnparsable { source })?;

    match PortCheck::of(&addr) {
        PortCheck::Available => Ok(Some(addr)),
        PortCheck::InUse => Err(FormFieldError::AddressPortInUse { port: addr.port() }),
        PortCheck::BindFailed(source) => Err(FormFieldError::AddressCannotBeBound { addr, source }),
    }
}

fn validate_required_address(raw: &str) -> Result<SocketAddr, FormFieldError> {
    validate_optional_address(raw)?.ok_or(FormFieldError::AddressRequired)
}

pub struct StartBalancerFormData {
    pub add_model_later: bool,
    pub balancer_address: String,
    pub balancer_address_error: Option<FormFieldError>,
    pub inference_address: String,
    pub inference_address_error: Option<FormFieldError>,
    pub inference_mode: InferenceMode,
    pub launch: BalancerLaunch,
    pub model_error: Option<FormFieldError>,
    pub selected_model: Option<ModelPreset>,
    pub web_admin_panel_address: String,
    pub web_admin_panel_address_error: Option<FormFieldError>,
    pub web_admin_panel_address_placeholder: String,
}

impl StartBalancerFormData {
    #[must_use]
    pub fn suggesting_addresses_on(interface_address: Option<IpAddr>) -> Self {
        let suggested_address_on = |port| {
            interface_address.map_or_else(String::new, |ip_address| {
                SocketAddr::new(ip_address, port).to_string()
            })
        };

        Self {
            add_model_later: false,
            balancer_address: suggested_address_on(BalancerDefaults::MANAGEMENT_PORT),
            balancer_address_error: None,
            inference_address: suggested_address_on(BalancerDefaults::INFERENCE_PORT),
            inference_address_error: None,
            inference_mode: InferenceMode::default(),
            launch: BalancerLaunch::NotRequested,
            model_error: None,
            selected_model: None,
            web_admin_panel_address: String::new(),
            web_admin_panel_address_error: None,
            web_admin_panel_address_placeholder: interface_address.map_or_else(
                || ADDRESS_PLACEHOLDER.to_owned(),
                |ip_address| {
                    SocketAddr::new(ip_address, BalancerDefaults::WEB_ADMIN_PANEL_PORT).to_string()
                },
            ),
        }
    }

    pub fn update(&mut self, message: StartBalancerFormMessage) -> StartBalancerFormAction {
        match message {
            StartBalancerFormMessage::SelectInferenceMode(InferenceModeChoice(inference_mode)) => {
                self.inference_mode = inference_mode;

                if self
                    .selected_model
                    .is_some_and(|preset| preset.inference_mode() != inference_mode)
                {
                    self.selected_model = None;
                }

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::SelectModel(preset) => {
                self.selected_model = Some(preset);
                self.model_error = None;

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::SetBalancerAddress(address) => {
                self.balancer_address = address;
                self.balancer_address_error = None;

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::SetInferenceAddress(address) => {
                self.inference_address = address;
                self.inference_address_error = None;

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::SetWebAdminPanelAddress(address) => {
                self.web_admin_panel_address = address;
                self.web_admin_panel_address_error = None;

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::ToggleAddModelLater(add_later) => {
                self.add_model_later = add_later;

                if add_later {
                    self.model_error = None;
                }

                StartBalancerFormAction::None
            }
            StartBalancerFormMessage::Confirm => self.validate_and_confirm(),
            StartBalancerFormMessage::Cancel => StartBalancerFormAction::Cancel,
        }
    }

    fn desired_state(&self) -> Result<BalancerDesiredState, FormFieldError> {
        if self.add_model_later {
            return Ok(BalancerDesiredState {
                inference_mode: self.inference_mode,
                ..BalancerDesiredState::default()
            });
        }

        self.selected_model
            .map(ModelPreset::to_balancer_desired_state)
            .ok_or(FormFieldError::ModelNotSelected)
    }

    fn validate_and_confirm(&mut self) -> StartBalancerFormAction {
        match (
            self.desired_state(),
            validate_required_address(&self.balancer_address),
            validate_required_address(&self.inference_address),
            validate_optional_address(&self.web_admin_panel_address),
        ) {
            (
                Ok(desired_state),
                Ok(management_addr),
                Ok(inference_addr),
                Ok(web_admin_panel_addr),
            ) => StartBalancerFormAction::StartBalancer(Box::new(balancer_runner_config(
                desired_state,
                management_addr,
                inference_addr,
                web_admin_panel_addr,
            ))),
            (desired_state, management_addr, inference_addr, web_admin_panel_addr) => {
                self.model_error = desired_state.err();
                self.balancer_address_error = management_addr.err();
                self.inference_address_error = inference_addr.err();
                self.web_admin_panel_address_error = web_admin_panel_addr.err();

                StartBalancerFormAction::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::net::TcpListener;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_state_database::state_database_type::StateDatabaseType;
    use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

    use super::StartBalancerFormData;
    use crate::balancer_launch::BalancerLaunch;
    use crate::form_field_error::FormFieldError;
    use crate::inference_mode_choice::InferenceModeChoice;
    use crate::model_preset::ModelPreset;
    use crate::start_balancer_form_action::StartBalancerFormAction;
    use crate::start_balancer_form_message::StartBalancerFormMessage;

    const UNASSIGNED_TEST_NET_ADDRESS: &str = "192.0.2.1:0";

    fn form_on_free_ports() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: EPHEMERAL_LOOPBACK_ADDR.to_string(),
            balancer_address_error: None,
            inference_address: EPHEMERAL_LOOPBACK_ADDR.to_string(),
            inference_address_error: None,
            inference_mode: InferenceMode::TextGeneration,
            launch: BalancerLaunch::NotRequested,
            model_error: None,
            selected_model: None,
            web_admin_panel_address: String::new(),
            web_admin_panel_address_error: None,
            web_admin_panel_address_placeholder: String::new(),
        }
    }

    fn multimodal_preset() -> ModelPreset {
        ModelPreset::ALL
            .into_iter()
            .find(|preset| preset.multimodal_projection() != AgentDesiredModel::None)
            .expect("a multimodal preset must be available")
    }

    #[test]
    fn requires_a_model_unless_it_is_added_later() {
        let mut form = form_on_free_ports();

        assert_eq!(
            discriminant(&form.update(StartBalancerFormMessage::Confirm)),
            discriminant(&StartBalancerFormAction::None)
        );
        assert_eq!(
            form.model_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::ModelNotSelected))
        );
    }

    #[test]
    fn starts_without_a_model_when_it_is_added_later() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));

        assert!(matches!(
            form.update(StartBalancerFormMessage::Confirm),
            StartBalancerFormAction::StartBalancer(runner_config)
                if matches!(
                    &runner_config.state_database_type,
                    StateDatabaseType::Memory(desired_state)
                        if **desired_state
                            == BalancerDesiredState::default()
                )
        ));
    }

    #[test]
    fn starts_a_cluster_without_a_model_in_the_selected_mode() {
        for inference_mode in [InferenceMode::Decision, InferenceMode::Embeddings] {
            let mut form = form_on_free_ports();

            form.update(StartBalancerFormMessage::SelectInferenceMode(
                InferenceModeChoice(inference_mode),
            ));
            form.update(StartBalancerFormMessage::ToggleAddModelLater(true));

            assert!(matches!(
                form.update(StartBalancerFormMessage::Confirm),
                StartBalancerFormAction::StartBalancer(runner_config)
                    if matches!(
                        &runner_config.state_database_type,
                        StateDatabaseType::Memory(desired_state)
                            if desired_state.inference_mode == inference_mode
                    )
            ));
        }
    }

    #[test]
    fn switching_the_mode_drops_a_preset_of_the_other_mode() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::SelectModel(
            ModelPreset::Qwen3_0_6B,
        ));
        form.update(StartBalancerFormMessage::SelectInferenceMode(
            InferenceModeChoice(InferenceMode::Embeddings),
        ));

        assert_eq!(form.selected_model, None);
    }

    #[test]
    fn switching_the_mode_keeps_a_preset_of_that_mode() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::SelectModel(
            ModelPreset::Qwen3_0_6B,
        ));
        form.update(StartBalancerFormMessage::SelectInferenceMode(
            InferenceModeChoice(InferenceMode::TextGeneration),
        ));

        assert_eq!(form.selected_model, Some(ModelPreset::Qwen3_0_6B));
    }

    #[test]
    fn requires_a_model_again_once_it_is_no_longer_added_later() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));
        form.update(StartBalancerFormMessage::ToggleAddModelLater(false));
        form.update(StartBalancerFormMessage::Confirm);

        assert_eq!(
            form.model_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::ModelNotSelected))
        );
    }

    #[cfg(feature = "web_admin_panel")]
    #[test]
    fn serves_no_web_admin_panel_without_its_address() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));

        assert!(matches!(
            form.update(StartBalancerFormMessage::Confirm),
            StartBalancerFormAction::StartBalancer(runner_config)
                if runner_config.web_admin_panel_service_configuration.is_none()
        ));
    }

    #[test]
    fn starts_with_the_selected_model_and_its_multimodal_projection() {
        let mut form = form_on_free_ports();
        let preset = multimodal_preset();
        let expected_model = AgentDesiredModel::HuggingFace(preset.model());
        let expected_multimodal_projection = preset.multimodal_projection();

        form.update(StartBalancerFormMessage::SelectModel(preset));

        assert!(matches!(
            form.update(StartBalancerFormMessage::Confirm),
            StartBalancerFormAction::StartBalancer(runner_config)
                if matches!(
                    &runner_config.state_database_type,
                    StateDatabaseType::Memory(desired_state)
                        if desired_state.model == expected_model
                            && desired_state.text_generation.multimodal.projection
                                == expected_multimodal_projection
                )
        ));
    }

    #[test]
    fn reports_every_invalid_address_at_once() {
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));
        form.update(StartBalancerFormMessage::SetBalancerAddress(String::new()));
        form.update(StartBalancerFormMessage::SetInferenceAddress(
            "not an address".to_owned(),
        ));
        form.update(StartBalancerFormMessage::SetWebAdminPanelAddress(
            "127.0.0.1".to_owned(),
        ));

        assert_eq!(
            discriminant(&form.update(StartBalancerFormMessage::Confirm)),
            discriminant(&StartBalancerFormAction::None)
        );
        assert_eq!(
            form.balancer_address_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::AddressRequired))
        );
        assert!(matches!(
            &form.inference_address_error,
            Some(FormFieldError::AddressUnparsable { source })
                if "not an address".parse::<SocketAddr>().err().as_ref() == Some(source)
        ));
        assert!(matches!(
            &form.web_admin_panel_address_error,
            Some(FormFieldError::AddressUnparsable { source })
                if "127.0.0.1".parse::<SocketAddr>().err().as_ref() == Some(source)
        ));
    }

    #[test]
    fn reports_an_address_another_listener_holds_as_in_use() {
        let competing_listener =
            TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("the competing listener must bind");
        let taken_address = competing_listener
            .local_addr()
            .expect("the competing listener must report its address");
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));
        form.update(StartBalancerFormMessage::SetInferenceAddress(
            taken_address.to_string(),
        ));

        assert_eq!(
            discriminant(&form.update(StartBalancerFormMessage::Confirm)),
            discriminant(&StartBalancerFormAction::None)
        );
        assert!(matches!(
            form.inference_address_error,
            Some(FormFieldError::AddressPortInUse { port }) if port == taken_address.port()
        ));
    }

    #[test]
    fn reports_why_an_address_cannot_be_bound() {
        let bind_error = TcpListener::bind(UNASSIGNED_TEST_NET_ADDRESS)
            .expect_err("an address assigned to no interface must not bind");
        let mut form = form_on_free_ports();

        form.update(StartBalancerFormMessage::ToggleAddModelLater(true));
        form.update(StartBalancerFormMessage::SetWebAdminPanelAddress(
            UNASSIGNED_TEST_NET_ADDRESS.to_owned(),
        ));

        assert_eq!(
            discriminant(&form.update(StartBalancerFormMessage::Confirm)),
            discriminant(&StartBalancerFormAction::None)
        );
        assert!(matches!(
            form.web_admin_panel_address_error,
            Some(FormFieldError::AddressCannotBeBound { addr, source })
                if addr.to_string() == UNASSIGNED_TEST_NET_ADDRESS
                    && source.kind() == bind_error.kind()
        ));
    }

    #[test]
    fn suggests_the_default_ports_on_the_detected_interface() {
        let form = StartBalancerFormData::suggesting_addresses_on(Some(IpAddr::V4(Ipv4Addr::new(
            192, 168, 1, 7,
        ))));

        assert_eq!(form.balancer_address, "192.168.1.7:8060");
        assert_eq!(form.inference_address, "192.168.1.7:8061");
        assert_eq!(form.web_admin_panel_address_placeholder, "192.168.1.7:8062");
    }

    #[test]
    fn suggests_no_address_without_a_detected_interface() {
        let form = StartBalancerFormData::suggesting_addresses_on(None);

        assert_eq!(form.balancer_address, "");
        assert_eq!(form.inference_address, "");
        assert_eq!(form.web_admin_panel_address_placeholder, "IP:port");
    }
}
