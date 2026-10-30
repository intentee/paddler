use std::path::Path;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Copy)]
pub enum DesiredModelFile {
    Model,
    MultimodalProjection,
    PointerHead,
}

impl DesiredModelFile {
    pub fn report_found(self, path: &Path, slot_aggregated_status: &SlotAggregatedStatus) {
        match self {
            Self::Model => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelFileExists(ModelPath::from(path)));
            }
            Self::MultimodalProjection => {}
            Self::PointerHead => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::PointerHeadFileExists(ModelPath::from(path)));
            }
        }
    }

    pub fn report_missing(self, path: &Path, slot_aggregated_status: &SlotAggregatedStatus) {
        let model_path = ModelPath::from(path);

        slot_aggregated_status.register_desired_state_issue(match self {
            Self::Model => AgentIssue::ModelFileDoesNotExist(model_path),
            Self::MultimodalProjection => {
                AgentIssue::MultimodalProjectionCannotBeLoaded(model_path)
            }
            Self::PointerHead => AgentIssue::PointerHeadCannotBeLoaded(model_path),
        });
    }
}
