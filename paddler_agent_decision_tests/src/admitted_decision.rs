use tokio::sync::mpsc;

use paddler_agent_decision::active_decision_request::ActiveDecisionRequest;
use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
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

    /// Advances the decision without a projector until it stops continuing.
    ///
    /// # Errors
    ///
    /// Returns the [`DecisionError`] of the advance that ended the decision.
    pub fn advance_until_it_ends(
        &mut self,
        decoder: &mut DecisionDecoder<'_, '_>,
    ) -> Result<DecisionAdvance, DecisionError> {
        loop {
            match self.active_decision_request.advance(decoder, None, None) {
                Ok(DecisionAdvance::Continuing) => {}
                final_advance => return final_advance,
            }
        }
    }
}
