use hf_hub::HFClient;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::huggingface_cache_location::HuggingFaceCacheLocation;
use crate::model_source_error::ModelSourceError;

#[derive(Clone, Debug)]
pub struct HuggingFaceHub {
    pub cache_location: HuggingFaceCacheLocation,
    pub endpoint: String,
}

impl HuggingFaceHub {
    pub fn start_client(
        &self,
        model_path: &ModelPath,
        slot_aggregated_status: &SlotAggregatedStatus,
    ) -> Result<HFClient, ModelSourceError> {
        match self
            .cache_location
            .client_builder()
            .endpoint(self.endpoint.clone())
            .build()
        {
            Ok(client) => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::HuggingFaceClientStarted(model_path.clone()));

                Ok(client)
            }
            Err(source) => {
                slot_aggregated_status.register_desired_state_issue(
                    AgentIssue::HuggingFaceClientCannotStart(model_path.clone()),
                );

                Err(ModelSourceError::HuggingFaceClientUnavailable(source))
            }
        }
    }
}
