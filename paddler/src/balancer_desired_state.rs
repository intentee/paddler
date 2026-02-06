use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;
use crate::agent_desired_state::AgentDesiredState;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::chat_template::ChatTemplate;
use crate::converts_to_applicable_state::ConvertsToApplicableState;
use crate::inference_parameters::InferenceParameters;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
    pub use_chat_template_override: bool,
}

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
