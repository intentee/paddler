use tokio::sync::mpsc;

use paddler_agent_decision::active_decision_request::ActiveDecisionRequest;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
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
        PreparedDecision {
            decision_result_rx,
            decision_stop_tx,
            request,
        }: PreparedDecision,
    ) -> Option<Self> {
        capacity_ledger
            .admit(request.plan.question_lane_demand())
            .map(|admission| Self {
                active_decision_request: ActiveDecisionRequest::new(request, admission),
                decision_result_rx,
                decision_stop_tx,
            })
    }
}
