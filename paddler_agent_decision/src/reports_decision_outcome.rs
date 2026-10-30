use tokio::sync::mpsc;

use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_messaging::decision_result::DecisionResult;

pub trait ReportsDecisionOutcome {
    fn report(
        self,
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    );
}

impl ReportsDecisionOutcome for DecisionResult {
    fn report(
        self,
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    ) {
        send_result_or_warn(agent_name, decision_result_tx, self);
    }
}
