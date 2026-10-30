use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_decision::prepared_decision_request::PreparedDecisionRequest;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_slots_for_leading_questions::decision_slots_for_leading_questions;

pub struct PreparedDecision {
    pub decision_result_rx: mpsc::UnboundedReceiver<DecisionResult>,
    pub decision_stop_tx: mpsc::UnboundedSender<()>,
    pub prepared_decision_request: PreparedDecisionRequest,
}

impl PreparedDecision {
    #[must_use]
    pub fn new(
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
        layout: DecisionTokenLayout,
    ) -> Self {
        let (decision_result_tx, decision_result_rx) = mpsc::unbounded_channel();
        let (decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();
        let reserved_slots = decision_slots_for_leading_questions(layout.remaining_questions.len());

        Self {
            decision_result_rx,
            decision_stop_tx,
            prepared_decision_request: PreparedDecisionRequest {
                decision_result_tx,
                decision_stop_rx,
                layout,
                slot_guard: SlotGuard::new(slot_aggregated_status, reserved_slots),
                started_at: Instant::now(),
            },
        }
    }
}
