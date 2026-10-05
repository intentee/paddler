use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
}

impl From<BalancerDesiredState> for BalancerApplicableState {
    fn from(
        BalancerDesiredState {
            chat_template_override,
            inference_parameters,
            model,
            multimodal_projection,
            use_chat_template_override,
        }: BalancerDesiredState,
    ) -> Self {
        Self {
            agent_desired_state: AgentDesiredState {
                chat_template_override: if use_chat_template_override {
                    chat_template_override
                } else {
                    None
                },
                inference_parameters,
                model,
                multimodal_projection,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::chat_template::ChatTemplate;

    use super::BalancerApplicableState;

    fn desired_state_with_override(use_chat_template_override: bool) -> BalancerDesiredState {
        BalancerDesiredState {
            chat_template_override: Some(ChatTemplate {
                content: "{{ messages }}".to_owned(),
            }),
            use_chat_template_override,
            ..BalancerDesiredState::default()
        }
    }

    #[test]
    fn applies_the_chat_template_override_when_it_is_enabled() {
        assert_eq!(
            BalancerApplicableState::from(desired_state_with_override(true))
                .agent_desired_state
                .chat_template_override,
            Some(ChatTemplate {
                content: "{{ messages }}".to_owned(),
            })
        );
    }

    #[test]
    fn ignores_the_chat_template_override_when_it_is_disabled() {
        assert_eq!(
            BalancerApplicableState::from(desired_state_with_override(false))
                .agent_desired_state
                .chat_template_override,
            None
        );
    }
}
