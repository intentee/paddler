use std::net::SocketAddr;

use crate::join_balancer_form_data::JoinBalancerFormData;

#[derive(Debug, Clone)]
pub enum Message {
    SetAgentName(String),
    SetBalancerAddress(String),
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

impl JoinBalancerFormData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SetAgentName(name) => {
                self.agent_name = name;

                Action::None
            }
            Message::SetBalancerAddress(address) => {
                self.balancer_address = address;
                self.balancer_address_error = None;

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
        self.balancer_address_error = None;
        self.slots_error = None;

        if self.balancer_address.is_empty() {
            self.balancer_address_error = Some("Cluster address is required.".to_owned());
        } else if self.balancer_address.parse::<SocketAddr>().is_err() {
            self.balancer_address_error =
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

        if self.balancer_address_error.is_some() || self.slots_error.is_some() {
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
            management_address: self.balancer_address.clone(),
            slots,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;

    use super::Action;
    use super::JoinBalancerFormData;
    use super::Message;

    #[test]
    fn set_agent_name_records_typed_value_into_form_state() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        match data.update(Message::SetAgentName("alice".to_owned())) {
            Action::None => {}
            _ => bail!("expected Action::None"),
        }

        if data.agent_name != "alice" {
            bail!("expected agent_name to record typed value");
        }

        Ok(())
    }

    #[test]
    fn set_balancer_address_clears_previously_set_address_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address_error: Some("stale".to_owned()),
            ..JoinBalancerFormData::default()
        };

        match data.update(Message::SetBalancerAddress("127.0.0.1:8080".to_owned())) {
            Action::None => {}
            _ => bail!("expected Action::None"),
        }

        if data.balancer_address_error.is_some() {
            bail!("expected balancer_address_error to be cleared");
        }

        if data.balancer_address != "127.0.0.1:8080" {
            bail!("expected new balancer_address to be stored");
        }

        Ok(())
    }

    #[test]
    fn set_slots_count_accepts_digit_only_input() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        let _action = data.update(Message::SetSlotsCount("42".to_owned()));

        if data.slots_count != "42" {
            bail!("expected slots_count to be updated to digit string");
        }

        Ok(())
    }

    #[test]
    fn set_slots_count_silently_ignores_non_digit_input() -> Result<()> {
        let mut data = JoinBalancerFormData {
            slots_count: "10".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _action = data.update(Message::SetSlotsCount("abc".to_owned()));

        if data.slots_count != "10" {
            bail!("expected slots_count to be unchanged after non-digit input");
        }

        Ok(())
    }

    #[test]
    fn cancel_message_returns_cancel_action() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        match data.update(Message::Cancel) {
            Action::Cancel => Ok(()),
            _ => bail!("expected Action::Cancel"),
        }
    }

    #[test]
    fn connecting_without_a_cluster_address_records_required_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            slots_count: "1".to_owned(),
            ..JoinBalancerFormData::default()
        };

        match data.update(Message::Connect) {
            Action::None => {}
            _ => bail!("expected Action::None when validation fails"),
        }

        match data.balancer_address_error.as_deref() {
            Some(message) if message.contains("required") => Ok(()),
            other => bail!("expected required-message error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_with_an_unparseable_cluster_address_records_invalid_format_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "not-a-socket-addr".to_owned(),
            slots_count: "1".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::Connect);

        match data.balancer_address_error.as_deref() {
            Some(message) if message.contains("IP:port") => Ok(()),
            other => bail!("expected IP:port-format error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_without_a_slot_count_records_required_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::Connect);

        match data.slots_error.as_deref() {
            Some(message) if message.contains("required") => Ok(()),
            other => bail!("expected required-slots error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_with_zero_slots_records_must_be_greater_than_zero_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            slots_count: "0".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::Connect);

        match data.slots_error.as_deref() {
            Some(message) if message.contains("greater than zero") => Ok(()),
            other => bail!("expected greater-than-zero error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_with_an_overflowing_slot_count_records_too_large_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            slots_count: "9999999999".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::Connect);

        match data.slots_error.as_deref() {
            Some(message) if message.contains("too large") => Ok(()),
            other => bail!("expected too-large error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_with_a_malformed_slot_count_falls_back_to_generic_invalid_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            slots_count: "abc".to_owned(),
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::Connect);

        match data.slots_error.as_deref() {
            Some(message) if message.contains("Invalid number of slots") => Ok(()),
            other => bail!("expected generic invalid-slots error, got {other:?}"),
        }
    }

    #[test]
    fn connecting_with_valid_input_and_no_agent_name_yields_connect_agent_with_name_none()
    -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            slots_count: "4".to_owned(),
            ..JoinBalancerFormData::default()
        };

        match data.update(Message::Connect) {
            Action::ConnectAgent {
                agent_name,
                management_address,
                slots,
            } => {
                if agent_name.is_some() {
                    bail!("expected agent_name to be None when field is empty");
                }
                if management_address != "127.0.0.1:8060" {
                    bail!("expected management_address to be forwarded verbatim");
                }
                if slots != 4 {
                    bail!("expected slots=4 to be forwarded");
                }
                Ok(())
            }
            _ => bail!("expected Action::ConnectAgent"),
        }
    }

    #[test]
    fn connecting_with_valid_input_and_a_filled_agent_name_yields_connect_agent_with_some_name()
    -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            agent_name: "primary-agent".to_owned(),
            slots_count: "2".to_owned(),
            ..JoinBalancerFormData::default()
        };

        match data.update(Message::Connect) {
            Action::ConnectAgent { agent_name, .. } => match agent_name.as_deref() {
                Some("primary-agent") => Ok(()),
                other => bail!("expected agent_name=Some(\"primary-agent\"), got {other:?}"),
            },
            _ => bail!("expected Action::ConnectAgent"),
        }
    }
}
