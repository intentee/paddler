use anyhow::Result;
use async_trait::async_trait;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::agent_desired_state::AgentDesiredState;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::converts_to_applicable_state::ConvertsToApplicableState;

#[async_trait]
impl ConvertsToApplicableState for BalancerDesiredState {
    type ApplicableState = BalancerApplicableState;
    type Context = ();

    async fn to_applicable_state(
        &self,
        _context: Self::Context,
    ) -> Result<Option<Self::ApplicableState>> {
        Ok(Some(BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: if self.use_chat_template_override {
                    self.chat_template_override.clone()
                } else {
                    None
                },
                inference_parameters: self.inference_parameters.clone(),
                model: self.model.clone(),
            },
        }))
    }
}
