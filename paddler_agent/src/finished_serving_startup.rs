use crate::agent_applicable_state::AgentApplicableState;
use crate::replacement_outcome::ReplacementOutcome;

pub struct FinishedServingStartup {
    pub agent_applicable_state: AgentApplicableState,
    pub outcome: ReplacementOutcome,
}
