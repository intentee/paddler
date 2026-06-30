use anyhow::Result;
use async_trait::async_trait;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_state_conversion::converts_to_applicable_state::ConvertsToApplicableState;
use paddler_state_conversion::converts_to_desired_state::ConvertsToDesiredState;

use crate::balancer_applicable_state::BalancerApplicableState;

pub struct BalancerDesiredStateConverter;

impl BalancerDesiredStateConverter {
    #[must_use]
    pub fn to_balancer_applicable_state(
        &self,
        desired_state: BalancerDesiredState,
    ) -> BalancerApplicableState {
        BalancerApplicableState {
            agent_desired_state: self.to_desired_state(desired_state),
        }
    }
}

impl ConvertsToDesiredState for BalancerDesiredStateConverter {
    type DesiredState = AgentDesiredState;
    type Source = BalancerDesiredState;

    fn to_desired_state(
        &self,
        BalancerDesiredState {
            chat_template_override,
            inference_parameters,
            model,
            multimodal_projection,
            use_chat_template_override,
        }: BalancerDesiredState,
    ) -> AgentDesiredState {
        AgentDesiredState {
            chat_template_override: if use_chat_template_override {
                chat_template_override
            } else {
                None
            },
            inference_parameters,
            model,
            multimodal_projection,
        }
    }
}

#[async_trait]
impl ConvertsToApplicableState for BalancerDesiredStateConverter {
    type ApplicableState = BalancerApplicableState;
    type DesiredState = BalancerDesiredState;

    async fn to_applicable_state(
        &self,
        desired_state: BalancerDesiredState,
    ) -> Result<BalancerApplicableState> {
        Ok(self.to_balancer_applicable_state(desired_state))
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::chat_template::ChatTemplate;

    use super::BalancerDesiredState;
    use super::BalancerDesiredStateConverter;
    use super::ConvertsToApplicableState;
    use super::ConvertsToDesiredState;

    fn desired_state_with_override(use_chat_template_override: bool) -> BalancerDesiredState {
        BalancerDesiredState {
            chat_template_override: Some(ChatTemplate {
                content: "custom template".to_owned(),
            }),
            use_chat_template_override,
            ..BalancerDesiredState::default()
        }
    }

    #[test]
    fn to_desired_state_keeps_chat_template_override_when_enabled() {
        let agent_desired_state =
            BalancerDesiredStateConverter.to_desired_state(desired_state_with_override(true));

        assert_eq!(
            agent_desired_state.chat_template_override,
            Some(ChatTemplate {
                content: "custom template".to_owned(),
            })
        );
    }

    #[test]
    fn to_desired_state_drops_chat_template_override_when_disabled() {
        let agent_desired_state =
            BalancerDesiredStateConverter.to_desired_state(desired_state_with_override(false));

        assert_eq!(agent_desired_state.chat_template_override, None);
    }

    #[tokio::test]
    async fn to_applicable_state_wraps_the_agent_desired_state() {
        let applicable_state = BalancerDesiredStateConverter
            .to_applicable_state(desired_state_with_override(true))
            .await
            .unwrap();

        assert_eq!(
            applicable_state.agent_desired_state.chat_template_override,
            Some(ChatTemplate {
                content: "custom template".to_owned(),
            })
        );
    }
}
