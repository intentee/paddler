use std::mem;

use paddler_ports::bound_port::BoundPort;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::address_field::AddressField;
use crate::model_preset::ModelPreset;
use crate::start_balancer_form_data::StartBalancerFormData;

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
        management_port: BoundPort,
        inference_port: BoundPort,
        web_admin_panel_port: Option<BoundPort>,
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
                self.balancer_address = AddressField::required_from_user_input(address);

                Action::None
            }
            Message::SetInferenceAddress(address) => {
                self.inference_address = AddressField::required_from_user_input(address);

                Action::None
            }
            Message::SetWebAdminPanelAddress(address) => {
                self.web_admin_panel_address = AddressField::optional_from_user_input(address);

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
        self.model_error = None;

        if !self.add_model_later && self.selected_model.is_none() {
            self.model_error = Some("Please select a model.".to_owned());
        }

        let balancer_address = mem::take(&mut self.balancer_address);
        let inference_address = mem::take(&mut self.inference_address);
        let web_admin_panel_address = mem::take(&mut self.web_admin_panel_address);

        let model_error_present = self.model_error.is_some();
        let any_required_address_invalid = matches!(
            balancer_address,
            AddressField::Empty | AddressField::Invalid { .. }
        ) || matches!(
            inference_address,
            AddressField::Empty | AddressField::Invalid { .. }
        );
        let web_admin_panel_invalid =
            matches!(web_admin_panel_address, AddressField::Invalid { .. });

        if model_error_present || any_required_address_invalid || web_admin_panel_invalid {
            self.balancer_address = match balancer_address {
                AddressField::Empty => AddressField::Invalid {
                    raw: String::new(),
                    error: "Address is required.".to_owned(),
                },
                other => other,
            };
            self.inference_address = match inference_address {
                AddressField::Empty => AddressField::Invalid {
                    raw: String::new(),
                    error: "Address is required.".to_owned(),
                },
                other => other,
            };
            self.web_admin_panel_address = web_admin_panel_address;
            return Action::None;
        }

        let AddressField::Bound {
            port: management_port,
            ..
        } = balancer_address
        else {
            return Action::None;
        };
        let AddressField::Bound {
            port: inference_port,
            ..
        } = inference_address
        else {
            return Action::None;
        };
        let web_admin_panel_port = match web_admin_panel_address {
            AddressField::Bound { port, .. } => Some(port),
            AddressField::Empty => None,
            AddressField::Invalid { .. } => return Action::None,
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
            management_port,
            inference_port,
            web_admin_panel_port,
            desired_state,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;
    use paddler_ports::bind_ephemeral_port::bind_ephemeral_port;
    use paddler_ports::bound_port::BoundPort;
    use paddler_types::agent_desired_model::AgentDesiredModel;

    use super::Action;
    use super::AddressField;
    use super::Message;
    use super::StartBalancerFormData;
    use crate::model_preset::ModelPreset;

    fn empty_form() -> StartBalancerFormData {
        StartBalancerFormData {
            add_model_later: false,
            balancer_address: AddressField::Empty,
            inference_address: AddressField::Empty,
            model_error: None,
            selected_model: None,
            starting: false,
            web_admin_panel_address: AddressField::Empty,
            web_admin_panel_address_placeholder: String::new(),
        }
    }

    fn first_preset() -> Result<ModelPreset> {
        ModelPreset::available_presets()
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("at least one preset must exist"))
    }

    fn bound_address_field() -> Result<AddressField> {
        let bound: BoundPort = bind_ephemeral_port()?;
        Ok(AddressField::Bound {
            raw: bound.socket_addr.to_string(),
            port: bound,
        })
    }

    #[test]
    fn set_balancer_address_with_unparseable_input_records_invalid_address() -> Result<()> {
        let mut data = empty_form();

        let _action = data.update(Message::SetBalancerAddress("not-a-socket-addr".to_owned()));

        assert!(matches!(
            data.balancer_address,
            AddressField::Invalid { .. }
        ));
        Ok(())
    }

    #[test]
    fn set_balancer_address_with_bindable_input_records_bound_listener() -> Result<()> {
        let bound = bind_ephemeral_port()?;
        let raw = bound.socket_addr.to_string();
        drop(bound);

        let mut data = empty_form();

        let _action = data.update(Message::SetBalancerAddress(raw));

        assert!(matches!(data.balancer_address, AddressField::Bound { .. }));
        Ok(())
    }

    #[test]
    fn set_balancer_address_with_empty_input_records_empty_state() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = AddressField::Invalid {
            raw: "stale".to_owned(),
            error: "stale".to_owned(),
        };

        let _action = data.update(Message::SetBalancerAddress(String::new()));

        assert!(matches!(data.balancer_address, AddressField::Empty));
        Ok(())
    }

    #[test]
    fn set_inference_address_with_empty_input_records_empty_state() -> Result<()> {
        let mut data = empty_form();
        let _action = data.update(Message::SetInferenceAddress(String::new()));
        assert!(matches!(data.inference_address, AddressField::Empty));
        Ok(())
    }

    #[test]
    fn set_web_admin_panel_address_with_empty_input_records_empty_state() -> Result<()> {
        let mut data = empty_form();
        let _action = data.update(Message::SetWebAdminPanelAddress(String::new()));
        assert!(matches!(data.web_admin_panel_address, AddressField::Empty));
        Ok(())
    }

    #[test]
    fn selecting_a_model_clears_a_previously_set_model_error() -> Result<()> {
        let mut data = empty_form();
        data.model_error = Some("stale".to_owned());

        let _ = data.update(Message::SelectModel(first_preset()?));

        assert!(
            data.model_error.is_none(),
            "expected model_error to be cleared after SelectModel"
        );

        Ok(())
    }

    #[test]
    fn toggling_add_model_later_on_clears_a_previously_set_model_error() -> Result<()> {
        let mut data = empty_form();
        data.model_error = Some("stale".to_owned());

        let _ = data.update(Message::ToggleAddModelLater(true));

        assert!(data.add_model_later);
        assert!(data.model_error.is_none());
        Ok(())
    }

    #[test]
    fn toggling_add_model_later_off_leaves_a_previously_set_model_error_in_place() -> Result<()> {
        let mut data = empty_form();
        data.add_model_later = true;
        data.model_error = Some("preserved".to_owned());

        let _ = data.update(Message::ToggleAddModelLater(false));

        assert!(!data.add_model_later);
        assert_eq!(data.model_error.as_deref(), Some("preserved"));
        Ok(())
    }

    #[test]
    fn cancel_message_returns_cancel_action() -> Result<()> {
        let mut data = empty_form();

        assert!(matches!(data.update(Message::Cancel), Action::Cancel));
        Ok(())
    }

    #[test]
    fn confirming_without_a_selected_model_records_a_model_required_error() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = bound_address_field()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(data.model_error.is_some());
        Ok(())
    }

    #[test]
    fn confirming_with_empty_balancer_address_records_a_required_error() -> Result<()> {
        let mut data = empty_form();
        data.inference_address = bound_address_field()?;
        data.selected_model = Some(first_preset()?);

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            data.balancer_address,
            AddressField::Invalid { .. }
        ));
        Ok(())
    }

    #[test]
    fn confirming_with_invalid_inference_address_keeps_the_invalid_state() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = AddressField::Invalid {
            raw: "not-a-socket".to_owned(),
            error: "Invalid address".to_owned(),
        };
        data.selected_model = Some(first_preset()?);

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            data.inference_address,
            AddressField::Invalid { .. }
        ));
        Ok(())
    }

    #[test]
    fn confirming_with_invalid_web_admin_panel_address_keeps_the_invalid_state() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = bound_address_field()?;
        data.web_admin_panel_address = AddressField::Invalid {
            raw: "not-a-socket".to_owned(),
            error: "Invalid address".to_owned(),
        };
        data.selected_model = Some(first_preset()?);

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            data.web_admin_panel_address,
            AddressField::Invalid { .. }
        ));
        Ok(())
    }

    #[test]
    fn confirming_with_valid_input_and_selected_model_returns_start_balancer_action() -> Result<()>
    {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = bound_address_field()?;
        data.selected_model = Some(first_preset()?);

        let action = data.update(Message::Confirm);

        assert!(matches!(
            action,
            Action::StartBalancer { desired_state, .. } if matches!(desired_state.model, AgentDesiredModel::HuggingFace(_))
        ));
        assert!(data.starting);

        Ok(())
    }

    #[test]
    fn confirming_with_add_model_later_returns_start_balancer_with_default_desired_state()
    -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = bound_address_field()?;
        data.add_model_later = true;

        let action = data.update(Message::Confirm);

        assert!(matches!(
            action,
            Action::StartBalancer { desired_state, .. } if matches!(desired_state.model, AgentDesiredModel::None)
        ));

        Ok(())
    }

    #[test]
    fn confirming_with_valid_web_admin_panel_address_carries_a_bound_port() -> Result<()> {
        let mut data = empty_form();
        data.balancer_address = bound_address_field()?;
        data.inference_address = bound_address_field()?;
        data.web_admin_panel_address = bound_address_field()?;
        data.selected_model = Some(first_preset()?);

        let action = data.update(Message::Confirm);

        assert!(matches!(
            action,
            Action::StartBalancer {
                web_admin_panel_port: Some(_),
                ..
            }
        ));

        Ok(())
    }
}
