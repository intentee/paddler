use std::net::SocketAddr;

use crate::join_cluster_config_data::JoinClusterConfigData;

#[derive(Debug, Clone)]
pub enum Message {
    SetAgentName(String),
    SetClusterAddress(String),
    SetSlotsCount(String),
    Connect,
    Cancel,
}

pub enum Action {
    None,
    Cancel,
    ConnectAgent {
        agent_name: Option<String>,
        management_address: String,
        slots: i32,
    },
}

impl JoinClusterConfigData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SetAgentName(name) => {
                self.agent_name = name;

                Action::None
            }
            Message::SetClusterAddress(address) => {
                self.cluster_address = address;
                self.cluster_address_error = None;

                Action::None
            }
            Message::SetSlotsCount(slots) => {
                if slots.is_empty() || slots.chars().all(|character| character.is_ascii_digit()) {
                    self.slots_count = slots;
                    self.slots_error = None;
                }

                Action::None
            }
            Message::Connect => self.validate_and_connect(),
            Message::Cancel => Action::Cancel,
        }
    }

    fn validate_and_connect(&mut self) -> Action {
        self.cluster_address_error = None;
        self.slots_error = None;

        if self.cluster_address.is_empty() {
            self.cluster_address_error = Some("Cluster address is required.".to_owned());
        } else if self.cluster_address.parse::<SocketAddr>().is_err() {
            self.cluster_address_error =
                Some("Invalid address, expected format: IP:port".to_owned());
        }

        let slots = if self.slots_count.is_empty() {
            self.slots_error = Some("Number of slots is required.".to_owned());
            None
        } else {
            match self.slots_count.parse::<i32>() {
                Ok(slots) if slots > 0 => Some(slots),
                Ok(non_positive_slots) => {
                    log::debug!("User entered non-positive slot count: {non_positive_slots}");
                    self.slots_error = Some(
                        "Invalid number of slots (the number should be greater than zero)."
                            .to_owned(),
                    );
                    None
                }
                Err(error) => {
                    let message = match error.kind() {
                        std::num::IntErrorKind::PosOverflow => "Number of slots is too large.",
                        unexpected_kind => {
                            log::error!("Unexpected slots parse error: {unexpected_kind:?}");
                            "Invalid number of slots."
                        }
                    };
                    self.slots_error = Some(message.to_owned());
                    None
                }
            }
        };

        if self.cluster_address_error.is_some() || self.slots_error.is_some() {
            return Action::None;
        }

        let Some(slots) = slots else {
            return Action::None;
        };

        let agent_name = if self.agent_name.is_empty() {
            None
        } else {
            Some(self.agent_name.clone())
        };

        Action::ConnectAgent {
            agent_name,
            management_address: self.cluster_address.clone(),
            slots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use super::JoinClusterConfigData;
    use super::Message;

    fn make_data() -> JoinClusterConfigData {
        JoinClusterConfigData::default()
    }

    #[test]
    fn set_agent_name_updates_field() {
        let mut data = make_data();

        let action = data.update(Message::SetAgentName("agent-1".to_owned()));

        assert!(matches!(action, Action::None));
        assert_eq!(data.agent_name, "agent-1");
    }

    #[test]
    fn set_cluster_address_clears_prior_error() {
        let mut data = make_data();
        data.cluster_address_error = Some("stale error".to_owned());

        let action = data.update(Message::SetClusterAddress("127.0.0.1:8060".to_owned()));

        assert!(matches!(action, Action::None));
        assert_eq!(data.cluster_address, "127.0.0.1:8060");
        assert!(data.cluster_address_error.is_none());
    }

    #[test]
    fn set_slots_count_accepts_digit_string() {
        let mut data = make_data();

        data.update(Message::SetSlotsCount("42".to_owned()));

        assert_eq!(data.slots_count, "42");
    }

    #[test]
    fn set_slots_count_rejects_non_digit_characters() {
        let mut data = make_data();
        data.slots_count = "7".to_owned();

        data.update(Message::SetSlotsCount("7a".to_owned()));

        assert_eq!(data.slots_count, "7");
    }

    #[test]
    fn connect_with_empty_cluster_address_sets_cluster_address_error() {
        let mut data = make_data();
        data.slots_count = "4".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.cluster_address_error.as_deref(),
            Some("Cluster address is required.")
        );
    }

    #[test]
    fn connect_with_invalid_cluster_address_sets_format_error() {
        let mut data = make_data();
        data.cluster_address = "not-an-address".to_owned();
        data.slots_count = "4".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.cluster_address_error.as_deref(),
            Some("Invalid address, expected format: IP:port")
        );
    }

    #[test]
    fn connect_with_empty_slots_sets_slots_error() {
        let mut data = make_data();
        data.cluster_address = "127.0.0.1:8060".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.slots_error.as_deref(),
            Some("Number of slots is required.")
        );
    }

    #[test]
    fn connect_with_zero_slots_sets_slots_error() {
        let mut data = make_data();
        data.cluster_address = "127.0.0.1:8060".to_owned();
        data.slots_count = "0".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert!(data.slots_error.is_some());
    }

    #[test]
    fn connect_with_overflowing_slots_sets_too_large_error() {
        let mut data = make_data();
        data.cluster_address = "127.0.0.1:8060".to_owned();
        data.slots_count = "99999999999999".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert_eq!(
            data.slots_error.as_deref(),
            Some("Number of slots is too large.")
        );
    }

    #[test]
    fn connect_with_valid_inputs_returns_connect_agent_action() {
        let mut data = make_data();
        data.cluster_address = "127.0.0.1:8060".to_owned();
        data.agent_name = "my-agent".to_owned();
        data.slots_count = "4".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(
            &action,
            Action::ConnectAgent {
                agent_name: Some(agent_name),
                management_address,
                slots: 4,
            } if agent_name == "my-agent" && management_address == "127.0.0.1:8060"
        ));
    }

    #[test]
    fn connect_with_empty_agent_name_produces_none_name() {
        let mut data = make_data();
        data.cluster_address = "127.0.0.1:8060".to_owned();
        data.slots_count = "4".to_owned();

        let action = data.update(Message::Connect);

        assert!(matches!(
            action,
            Action::ConnectAgent {
                agent_name: None,
                ..
            }
        ));
    }

    #[test]
    fn cancel_returns_cancel_action() {
        let mut data = make_data();

        let action = data.update(Message::Cancel);

        assert!(matches!(action, Action::Cancel));
    }
}
