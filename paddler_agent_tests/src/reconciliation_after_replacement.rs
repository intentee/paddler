use std::sync::Arc;

use paddler_agent::reconciled_state_holder::ReconciledStateHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

pub struct ReconciliationAfterReplacement {
    pub reconciled_state_holder: Arc<ReconciledStateHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}
