use std::net::SocketAddr;
use std::num::IntErrorKind;

use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
use paddler_agent_runner::llama_cpp_max_sequences::LLAMA_CPP_MAX_SEQUENCES;

use crate::form_field_error::FormFieldError;
use crate::join_balancer_form_action::JoinBalancerFormAction;
use crate::join_balancer_form_message::JoinBalancerFormMessage;

#[derive(Default)]
pub struct JoinBalancerFormData {
    pub agent_name: String,
    pub balancer_address: String,
    pub balancer_address_error: Option<FormFieldError>,
    pub slots_count: String,
    pub slots_error: Option<FormFieldError>,
}

impl JoinBalancerFormData {
    #[must_use]
    pub fn entered_agent_name(&self) -> Option<String> {
        (!self.agent_name.is_empty()).then(|| self.agent_name.clone())
    }

    pub fn update(&mut self, message: JoinBalancerFormMessage) -> JoinBalancerFormAction {
        match message {
            JoinBalancerFormMessage::SetAgentName(name) => {
                self.agent_name = name;

                JoinBalancerFormAction::None
            }
            JoinBalancerFormMessage::SetBalancerAddress(address) => {
                self.balancer_address = address;
                self.balancer_address_error = None;

                JoinBalancerFormAction::None
            }
            JoinBalancerFormMessage::SetSlotsCount(slots) => {
                if slots.is_empty() || slots.chars().all(|character| character.is_ascii_digit()) {
                    self.slots_count = slots;
                    self.slots_error = None;
                }

                JoinBalancerFormAction::None
            }
            JoinBalancerFormMessage::Connect => self.validate_and_connect(),
            JoinBalancerFormMessage::Cancel => JoinBalancerFormAction::Cancel,
        }
    }

    fn validated_balancer_address(&self) -> Result<String, FormFieldError> {
        if self.balancer_address.is_empty() {
            return Err(FormFieldError::ClusterAddressRequired);
        }

        self.balancer_address
            .parse::<SocketAddr>()
            .map(|_balancer_address| self.balancer_address.clone())
            .map_err(|source| FormFieldError::AddressUnparsable { source })
    }

    fn validated_slots(&self) -> Result<u16, FormFieldError> {
        match self.slots_count.parse::<u16>() {
            Ok(0) => Err(FormFieldError::SlotsNotPositive),
            Ok(slots) if slots <= LLAMA_CPP_MAX_SEQUENCES => Ok(slots),
            Ok(_slots_above_limit) => Err(FormFieldError::SlotsAboveLimit {
                limit: LLAMA_CPP_MAX_SEQUENCES,
            }),
            Err(source) => Err(match source.kind() {
                IntErrorKind::Empty => FormFieldError::SlotsRequired,
                IntErrorKind::PosOverflow => FormFieldError::SlotsAboveLimit {
                    limit: LLAMA_CPP_MAX_SEQUENCES,
                },
                _other_kind => FormFieldError::SlotsInvalid { source },
            }),
        }
    }

    fn validate_and_connect(&mut self) -> JoinBalancerFormAction {
        match (self.validated_balancer_address(), self.validated_slots()) {
            (Ok(management_address), Ok(slots)) => {
                JoinBalancerFormAction::ConnectAgent(AgentRunnerConfig {
                    agent_name: self.entered_agent_name(),
                    management_address,
                    slots,
                })
            }
            (management_address, slots) => {
                self.balancer_address_error = management_address.err();
                self.slots_error = slots.err();

                JoinBalancerFormAction::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::net::SocketAddr;

    use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
    use paddler_agent_runner::llama_cpp_max_sequences::LLAMA_CPP_MAX_SEQUENCES;

    use super::JoinBalancerFormData;
    use crate::form_field_error::FormFieldError;
    use crate::join_balancer_form_action::JoinBalancerFormAction;
    use crate::join_balancer_form_message::JoinBalancerFormMessage;

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

        form.update(JoinBalancerFormMessage::SetSlotsCount("4a".to_owned()));

        assert_eq!(form.slots_count, "4");
    }

    #[test]
    fn connects_with_the_entered_address_slots_and_agent_name() {
        let mut form = form_with("127.0.0.1:8060", "4");

        form.update(JoinBalancerFormMessage::SetAgentName("gpu-box".to_owned()));

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::ConnectAgent(AgentRunnerConfig {
                agent_name: Some("gpu-box".to_owned()),
                management_address: "127.0.0.1:8060".to_owned(),
                slots: 4,
            })
        );
    }

    #[test]
    fn connects_without_an_agent_name_when_none_was_entered() {
        let mut form = form_with("127.0.0.1:8060", "1");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::ConnectAgent(AgentRunnerConfig {
                agent_name: None,
                management_address: "127.0.0.1:8060".to_owned(),
                slots: 1,
            })
        );
    }

    #[test]
    fn requires_both_the_address_and_the_slot_count() {
        let mut form = form_with("", "");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert_eq!(
            form.balancer_address_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::ClusterAddressRequired))
        );
        assert_eq!(
            form.slots_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::SlotsRequired))
        );
    }

    #[test]
    fn rejects_an_address_without_a_port() {
        let mut form = form_with("127.0.0.1", "1");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert!(matches!(
            &form.balancer_address_error,
            Some(FormFieldError::AddressUnparsable { source })
                if "127.0.0.1".parse::<SocketAddr>().err().as_ref() == Some(source)
        ));
    }

    #[test]
    fn rejects_zero_slots() {
        let mut form = form_with("127.0.0.1:8060", "0");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert_eq!(
            form.slots_error.as_ref().map(discriminant),
            Some(discriminant(&FormFieldError::SlotsNotPositive))
        );
    }

    #[test]
    fn rejects_a_slot_count_that_does_not_fit() {
        let mut form = form_with("127.0.0.1:8060", "99999999999");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert!(matches!(
            form.slots_error,
            Some(FormFieldError::SlotsAboveLimit { limit }) if limit == LLAMA_CPP_MAX_SEQUENCES
        ));
    }

    #[test]
    fn rejects_a_slot_count_above_what_llama_cpp_can_serve() {
        let mut form = form_with("127.0.0.1:8060", "257");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert!(matches!(
            form.slots_error,
            Some(FormFieldError::SlotsAboveLimit { limit }) if limit == LLAMA_CPP_MAX_SEQUENCES
        ));
    }

    #[test]
    fn rejects_a_slot_count_that_is_not_a_number() {
        let mut form = form_with("127.0.0.1:8060", "+");

        assert_eq!(
            form.update(JoinBalancerFormMessage::Connect),
            JoinBalancerFormAction::None
        );
        assert!(matches!(
            &form.slots_error,
            Some(FormFieldError::SlotsInvalid { source })
                if "+".parse::<u16>().err().as_ref() == Some(source)
        ));
    }

    #[test]
    fn clears_the_address_error_when_the_address_changes() {
        let mut form = form_with("", "1");

        form.update(JoinBalancerFormMessage::Connect);
        form.update(JoinBalancerFormMessage::SetBalancerAddress(
            "127.0.0.1:8060".to_owned(),
        ));

        assert!(form.balancer_address_error.is_none());
    }
}
