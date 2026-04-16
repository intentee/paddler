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

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use anyhow::Context as _;
    use anyhow::Result;

    use super::Action;
    use super::Message;
    use super::StartClusterConfigData;
    use crate::model_preset::ModelPreset;

    fn make_data() -> StartClusterConfigData {
        StartClusterConfigData {
            cluster_address: String::new(),
            cluster_address_error: None,
            inference_address: String::new(),
            inference_address_error: None,
            model_error: None,
            selected_model: None,
            starting: false,
        }
    }

    fn first_preset() -> Result<ModelPreset> {
        ModelPreset::available_presets()
            .into_iter()
            .next()
            .context("available_presets must expose at least one model")
    }

    fn ephemeral_local_addr() -> Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        drop(listener);

        Ok(format!("127.0.0.1:{port}"))
    }

    #[test]
    fn select_model_sets_model_and_clears_error() -> Result<()> {
        let mut data = make_data();
        data.model_error = Some("pick one".to_owned());

        let action = data.update(Message::SelectModel(first_preset()?));

        assert!(matches!(action, Action::None));
        assert!(data.selected_model.is_some());
        assert!(data.model_error.is_none());

        Ok(())
    }

    #[test]
    fn set_cluster_address_clears_prior_error() {
        let mut data = make_data();
        data.cluster_address_error = Some("stale".to_owned());

        let action = data.update(Message::SetClusterAddress("127.0.0.1:8060".to_owned()));

        assert!(matches!(action, Action::None));
        assert_eq!(data.cluster_address, "127.0.0.1:8060");
        assert!(data.cluster_address_error.is_none());
    }

    #[test]
    fn set_inference_address_clears_prior_error() {
        let mut data = make_data();
        data.inference_address_error = Some("stale".to_owned());

        let action = data.update(Message::SetInferenceAddress("127.0.0.1:8061".to_owned()));

        assert!(matches!(action, Action::None));
        assert_eq!(data.inference_address, "127.0.0.1:8061");
        assert!(data.inference_address_error.is_none());
    }

    #[test]
    fn confirm_without_model_sets_model_error() -> Result<()> {
        let mut data = make_data();
        data.cluster_address = ephemeral_local_addr()?;
        data.inference_address = ephemeral_local_addr()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(data.model_error.is_some());

        Ok(())
    }

    #[test]
    fn confirm_with_empty_cluster_address_sets_cluster_address_error() -> Result<()> {
        let mut data = make_data();
        data.selected_model = Some(first_preset()?);
        data.inference_address = ephemeral_local_addr()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.cluster_address_error.as_deref(),
            Some("Cluster address is required.")
        );

        Ok(())
    }

    #[test]
    fn confirm_with_invalid_cluster_address_sets_format_error() -> Result<()> {
        let mut data = make_data();
        data.selected_model = Some(first_preset()?);
        data.cluster_address = "not-an-address".to_owned();
        data.inference_address = ephemeral_local_addr()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.cluster_address_error.as_deref(),
            Some("Invalid address, expected format: IP:port")
        );

        Ok(())
    }

    #[test]
    fn confirm_with_port_already_in_use_sets_error_mentioning_port() -> Result<()> {
        let busy_listener = TcpListener::bind("127.0.0.1:0")?;
        let busy_port = busy_listener.local_addr()?.port();
        let mut data = make_data();
        data.selected_model = Some(first_preset()?);
        data.cluster_address = format!("127.0.0.1:{busy_port}");
        data.inference_address = ephemeral_local_addr()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::None));
        assert!(
            data.cluster_address_error
                .as_deref()
                .is_some_and(|message| message.contains(&busy_port.to_string()))
        );

        drop(busy_listener);

        Ok(())
    }

    #[test]
    fn confirm_with_valid_inputs_returns_start_cluster_action_and_marks_starting() -> Result<()> {
        let mut data = make_data();
        data.selected_model = Some(first_preset()?);
        data.cluster_address = ephemeral_local_addr()?;
        data.inference_address = ephemeral_local_addr()?;

        let action = data.update(Message::Confirm);

        assert!(matches!(action, Action::StartCluster { .. }));
        assert!(data.starting);

        Ok(())
    }

    #[test]
    fn cancel_returns_cancel_action() {
        let mut data = make_data();

        let action = data.update(Message::Cancel);

        assert!(matches!(action, Action::Cancel));
    }
}
