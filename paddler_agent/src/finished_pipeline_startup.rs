use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_error::AgentError;
use crate::started_pipeline::StartedPipeline;

pub struct FinishedPipelineStartup {
    pub agent_applicable_state: AgentApplicableState,
    pub outcome: Result<StartedPipeline, AgentError>,
}
