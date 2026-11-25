use tokio::sync::mpsc;

use paddler_agent_decision::scheduled_decision_request::ScheduledDecisionRequest;
use paddler_messaging::decision_result::DecisionResult;

pub struct PreparedDecision {
    pub decision_result_rx: mpsc::UnboundedReceiver<DecisionResult>,
    pub decision_stop_tx: mpsc::UnboundedSender<()>,
    pub request: ScheduledDecisionRequest,
}
