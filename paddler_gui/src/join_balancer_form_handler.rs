use std::mem;

use crate::connect_address_field::ConnectAddressField;
use crate::join_balancer_form_data::JoinBalancerFormData;
use crate::slot_count_field::SlotCountField;

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
                self.balancer_address = ConnectAddressField::from_user_input(address);

                Action::None
            }
            Message::SetSlotsCount(slots) => {
                self.slots_count = SlotCountField::from_user_input(slots);

                Action::None
            }
            Message::Connect => self.validate_and_connect(),
            Message::Cancel => Action::Cancel,
        }
    }

    fn validate_and_connect(&mut self) -> Action {
        let balancer_address = mem::take(&mut self.balancer_address);
        let slots_count = mem::take(&mut self.slots_count);

        let required_balancer_address = match balancer_address {
            ConnectAddressField::Empty => ConnectAddressField::Invalid {
                raw: String::new(),
                error: "Cluster address is required.".to_owned(),
            },
            other => other,
        };
        let required_slots_count = match slots_count {
            SlotCountField::Empty => SlotCountField::Invalid {
                raw: String::new(),
                error: "Number of slots is required.".to_owned(),
            },
            other => other,
        };

        let address_valid = matches!(required_balancer_address, ConnectAddressField::Valid { .. });
        let slots_valid = matches!(required_slots_count, SlotCountField::Valid { .. });

        if !address_valid || !slots_valid {
            self.balancer_address = required_balancer_address;
            self.slots_count = required_slots_count;
            return Action::None;
        }

        let ConnectAddressField::Valid {
            raw: address_raw, ..
        } = required_balancer_address
        else {
            return Action::None;
        };
        let SlotCountField::Valid { value: slots, .. } = required_slots_count else {
            return Action::None;
        };

        let agent_name = if self.agent_name.is_empty() {
            None
        } else {
            Some(self.agent_name.clone())
        };

        Action::ConnectAgent {
            agent_name,
            management_address: address_raw,
            slots,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use std::net::SocketAddr;
    use std::net::TcpListener;

    use anyhow::Result;

    use super::Action;
    use super::ConnectAddressField;
    use super::JoinBalancerFormData;
    use super::Message;
    use super::SlotCountField;

    fn valid_field(raw: &str) -> Result<ConnectAddressField> {
        let socket_addr: SocketAddr = raw.parse()?;
        Ok(ConnectAddressField::Valid {
            raw: raw.to_owned(),
            socket_addr,
        })
    }

    #[test]
    fn set_agent_name_records_typed_value_into_form_state() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        assert!(matches!(
            data.update(Message::SetAgentName("alice".to_owned())),
            Action::None
        ));
        assert_eq!(data.agent_name, "alice");

        Ok(())
    }

    #[test]
    fn set_balancer_address_with_unparseable_input_records_invalid_state() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        let _ = data.update(Message::SetBalancerAddress("not-a-socket-addr".to_owned()));

        assert!(matches!(
            data.balancer_address,
            ConnectAddressField::Invalid { .. }
        ));

        Ok(())
    }

    #[test]
    fn set_balancer_address_with_empty_input_records_empty_state() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: ConnectAddressField::Invalid {
                raw: "stale".to_owned(),
                error: "stale".to_owned(),
            },
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::SetBalancerAddress(String::new()));

        assert!(matches!(data.balancer_address, ConnectAddressField::Empty));

        Ok(())
    }

    #[test]
    fn set_balancer_address_with_already_bound_port_still_records_valid_state() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let mut data = JoinBalancerFormData::default();
        let _ = data.update(Message::SetBalancerAddress(addr.to_string()));

        assert!(matches!(
            data.balancer_address,
            ConnectAddressField::Valid { .. }
        ));

        Ok(())
    }

    #[test]
    fn set_slots_count_accepts_digit_input() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        let _ = data.update(Message::SetSlotsCount("42".to_owned()));

        assert!(matches!(data.slots_count, SlotCountField::Valid { .. }));

        Ok(())
    }

    #[test]
    fn set_slots_count_with_non_digit_input_records_invalid_state() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        let _ = data.update(Message::SetSlotsCount("abc".to_owned()));

        assert!(matches!(data.slots_count, SlotCountField::Invalid { .. }));

        Ok(())
    }

    #[test]
    fn set_slots_count_with_empty_input_records_empty_state() -> Result<()> {
        let mut data = JoinBalancerFormData {
            slots_count: SlotCountField::Valid {
                raw: "10".to_owned(),
                value: 10,
            },
            ..JoinBalancerFormData::default()
        };

        let _ = data.update(Message::SetSlotsCount(String::new()));

        assert!(matches!(data.slots_count, SlotCountField::Empty));

        Ok(())
    }

    #[test]
    fn cancel_message_returns_cancel_action() -> Result<()> {
        let mut data = JoinBalancerFormData::default();

        assert!(matches!(data.update(Message::Cancel), Action::Cancel));

        Ok(())
    }

    #[test]
    fn connecting_without_a_cluster_address_records_required_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            slots_count: SlotCountField::Valid {
                raw: "1".to_owned(),
                value: 1,
            },
            ..JoinBalancerFormData::default()
        };

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            &data.balancer_address,
            ConnectAddressField::Invalid { error, .. } if error.contains("required")
        ));

        Ok(())
    }

    #[test]
    fn connecting_without_a_slot_count_records_required_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: valid_field("127.0.0.1:9001")?,
            ..JoinBalancerFormData::default()
        };

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            &data.slots_count,
            SlotCountField::Invalid { error, .. } if error.contains("required")
        ));

        Ok(())
    }

    #[test]
    fn connecting_with_a_zero_slot_count_records_must_be_greater_than_zero_error() -> Result<()> {
        let mut data = JoinBalancerFormData {
            balancer_address: valid_field("127.0.0.1:9002")?,
            slots_count: SlotCountField::from_user_input("0".to_owned()),
            ..JoinBalancerFormData::default()
        };

        let action = data.update(Message::Connect);

        assert!(matches!(action, Action::None));
        assert!(matches!(
            &data.slots_count,
            SlotCountField::Invalid { error, .. } if error.contains("greater than zero")
        ));

        Ok(())
    }

    #[test]
    fn connecting_with_valid_input_and_no_agent_name_yields_connect_agent_with_name_none()
    -> Result<()> {
        let raw_address = "127.0.0.1:9003".to_owned();
        let mut data = JoinBalancerFormData {
            balancer_address: valid_field(&raw_address)?,
            slots_count: SlotCountField::Valid {
                raw: "4".to_owned(),
                value: 4,
            },
            ..JoinBalancerFormData::default()
        };

        let action = data.update(Message::Connect);

        assert!(matches!(
            action,
            Action::ConnectAgent {
                agent_name: None,
                ref management_address,
                slots: 4,
            } if management_address == &raw_address
        ));

        Ok(())
    }

    #[test]
    fn connecting_with_valid_input_and_a_filled_agent_name_yields_connect_agent_with_some_name()
    -> Result<()> {
        let mut data = JoinBalancerFormData {
            agent_name: "primary".to_owned(),
            balancer_address: valid_field("127.0.0.1:9004")?,
            slots_count: SlotCountField::Valid {
                raw: "2".to_owned(),
                value: 2,
            },
        };

        let action = data.update(Message::Connect);

        assert!(matches!(
            action,
            Action::ConnectAgent { ref agent_name, .. } if agent_name.as_deref() == Some("primary")
        ));

        Ok(())
    }
}
