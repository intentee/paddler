use anyhow::Result;
use async_trait::async_trait;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_state_conversion::converts_to_applicable_state::ConvertsToApplicableState;
use paddler_state_conversion::converts_to_desired_state::ConvertsToDesiredState;

use crate::balancer_applicable_state::BalancerApplicableState;

pub struct BalancerDesiredStateConverter;

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
        Ok(BalancerApplicableState {
            agent_desired_state: self.to_desired_state(desired_state),
        })
    }
}
