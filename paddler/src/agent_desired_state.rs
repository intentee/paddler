use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_desired_model::AgentDesiredModel;
use crate::chat_template::ChatTemplate;
use crate::converts_to_applicable_state::ConvertsToApplicableState;
use crate::inference_parameters::InferenceParameters;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
}

#[async_trait]
impl ConvertsToApplicableState for AgentDesiredState {
    type ApplicableState = AgentApplicableState;
    type Context = Arc<SlotAggregatedStatus>;

    async fn to_applicable_state(
        &self,
        slot_aggregated_status: Self::Context,
    ) -> Result<Option<Self::ApplicableState>> {
        Ok(Some(AgentApplicableState {
            chat_template_override: self.chat_template_override.clone(),
            inference_parameters: self.inference_parameters.clone(),
            model_path: self
                .model
                .to_applicable_state(slot_aggregated_status)
                .await?,
        }))
    }
}
