use std::sync::Arc;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

pub struct ReconciliationAfterReplacement {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}
