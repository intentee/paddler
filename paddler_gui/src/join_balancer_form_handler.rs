use std::net::SocketAddr;
use std::num::IntErrorKind;

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

    fn validated_balancer_address(&self) -> Result<String, String> {
        if self.balancer_address.is_empty() {
            return Err("Cluster address is required.".to_owned());
        }

        match self.balancer_address.parse::<SocketAddr>() {
            Ok(_balancer_address) => Ok(self.balancer_address.clone()),
            Err(error) => Err(format!(
                "Invalid address ({error}), expected format: IP:port"
            )),
        }
    }

    fn validated_slots(&self) -> Result<i32, String> {
        match self.slots_count.parse::<i32>() {
            Ok(slots) if slots > 0 => Ok(slots),
            Ok(non_positive_slots) => {
                log::debug!("User entered non-positive slot count: {non_positive_slots}");

                Err("Invalid number of slots (the number should be greater than zero).".to_owned())
            }
            Err(error) => Err(match error.kind() {
                IntErrorKind::Empty => "Number of slots is required.",
                IntErrorKind::PosOverflow => "Number of slots is too large.",
                unexpected_kind => {
                    log::error!("Unexpected slots parse error: {unexpected_kind:?}");

                    "Invalid number of slots."
                }
            }
            .to_owned()),
        }
    }

    fn validate_and_connect(&mut self) -> Action {
        let management_address = self.validated_balancer_address();
        let slots = self.validated_slots();

        self.balancer_address_error = management_address.as_ref().err().cloned();
        self.slots_error = slots.as_ref().err().cloned();

        let (Ok(management_address), Ok(slots)) = (management_address, slots) else {
            return Action::None;
        };

        Action::ConnectAgent {
            agent_name: (!self.agent_name.is_empty()).then(|| self.agent_name.clone()),
            management_address,
            slots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use super::Message;
    use crate::join_balancer_form_data::JoinBalancerFormData;

    fn form_with(balancer_address: &str, slots_count: &str) -> JoinBalancerFormData {
        JoinBalancerFormData {
            balancer_address: balancer_address.to_owned(),
            slots_count: slots_count.to_owned(),
            ..JoinBalancerFormData::default()
        }
    }

    #[test]
    fn ignores_slot_input_that_is_not_a_number() {
        let mut form = form_with("", "4");

        form.update(Message::SetSlotsCount("4a".to_owned()));

        assert_eq!(form.slots_count, "4");
    }

    #[test]
    fn connects_with_the_entered_address_slots_and_agent_name() {
        let mut form = form_with("127.0.0.1:8060", "4");

        form.update(Message::SetAgentName("gpu-box".to_owned()));

        assert!(matches!(
            form.update(Message::Connect),
            Action::ConnectAgent {
                agent_name: Some(agent_name),
                management_address,
                slots: 4,
            } if agent_name == "gpu-box" && management_address == "127.0.0.1:8060"
        ));
    }

    #[test]
    fn connects_without_an_agent_name_when_none_was_entered() {
        let mut form = form_with("127.0.0.1:8060", "1");

        assert!(matches!(
            form.update(Message::Connect),
            Action::ConnectAgent {
                agent_name: None,
                ..
            }
        ));
    }

    #[test]
    fn requires_both_the_address_and_the_slot_count() {
        let mut form = form_with("", "");

        assert!(matches!(form.update(Message::Connect), Action::None));
        assert_eq!(
            form.balancer_address_error.as_deref(),
            Some("Cluster address is required.")
        );
        assert_eq!(
            form.slots_error.as_deref(),
            Some("Number of slots is required.")
        );
    }

    #[test]
    fn rejects_an_address_without_a_port() {
        let mut form = form_with("127.0.0.1", "1");

        assert!(matches!(form.update(Message::Connect), Action::None));
        assert_eq!(
            form.balancer_address_error.as_deref(),
            Some("Invalid address (invalid socket address syntax), expected format: IP:port")
        );
    }

    #[test]
    fn rejects_zero_slots() {
        let mut form = form_with("127.0.0.1:8060", "0");

        assert!(matches!(form.update(Message::Connect), Action::None));
        assert_eq!(
            form.slots_error.as_deref(),
            Some("Invalid number of slots (the number should be greater than zero).")
        );
    }

    #[test]
    fn rejects_a_slot_count_that_does_not_fit() {
        let mut form = form_with("127.0.0.1:8060", "99999999999");

        assert!(matches!(form.update(Message::Connect), Action::None));
        assert_eq!(
            form.slots_error.as_deref(),
            Some("Number of slots is too large.")
        );
    }

    #[test]
    fn rejects_a_slot_count_that_is_not_a_number() {
        let mut form = form_with("127.0.0.1:8060", "+");

        assert!(matches!(form.update(Message::Connect), Action::None));
        assert_eq!(
            form.slots_error.as_deref(),
            Some("Invalid number of slots.")
        );
    }

    #[test]
    fn clears_the_address_error_when_the_address_changes() {
        let mut form = form_with("", "1");

        form.update(Message::Connect);
        form.update(Message::SetBalancerAddress("127.0.0.1:8060".to_owned()));

        assert_eq!(form.balancer_address_error, None);
    }
}
