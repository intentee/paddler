use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
pub use paddler_types::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state::AgentApplicableState;
use crate::converts_to_applicable_state::ConvertsToApplicableState;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[async_trait]
impl ConvertsToApplicableState for AgentDesiredState {
    type ApplicableState = AgentApplicableState;
    type Context = Arc<SlotAggregatedStatus>;

    async fn to_applicable_state(
        &self,
        slot_aggregated_status: Self::Context,
    ) -> Result<Option<Self::ApplicableState>> {
        let model_path = self
            .model
            .to_applicable_state(slot_aggregated_status.clone())
            .await?;
        let multimodal_projection_path = self
            .multimodal_projection
            .to_applicable_state(slot_aggregated_status)
            .await?;

        Ok(Some(AgentApplicableState {
            chat_template_override: self.chat_template_override.clone(),
            inference_parameters: self.inference_parameters.clone(),
            model_path,
            multimodal_projection_path,
        }))
    }
}
