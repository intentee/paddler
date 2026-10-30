use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_decision::active_decision_request::ActiveDecisionRequest;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::decision_result::DecisionResult;

use crate::prepared_decision::PreparedDecision;

pub struct AdmittedDecision {
    pub active_decision_request: ActiveDecisionRequest,
    pub decision_result_rx: mpsc::UnboundedReceiver<DecisionResult>,
    pub decision_stop_tx: mpsc::UnboundedSender<()>,
}

impl AdmittedDecision {
    #[must_use]
    pub fn admit(
        capacity_ledger: &DecisionCapacityLedger,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
        layout: DecisionTokenLayout,
    ) -> Option<Self> {
        let PreparedDecision {
            decision_result_rx,
            decision_stop_tx,
            prepared_decision_request,
        } = PreparedDecision::new(slot_aggregated_status, layout);
        capacity_ledger
            .admit(prepared_decision_request.layout.question_lane_demand())
            .map(|admission| Self {
                active_decision_request: ActiveDecisionRequest::new(
                    prepared_decision_request,
                    admission,
                ),
                decision_result_rx,
                decision_stop_tx,
            })
    }
}
