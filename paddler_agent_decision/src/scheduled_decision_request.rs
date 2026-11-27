use std::time::Instant;

use tokio::sync::mpsc;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::decision_result::DecisionResult;

use crate::decision_error::DecisionError;
use crate::decision_plan::DecisionPlan;
use crate::reports_decision_outcome::ReportsDecisionOutcome as _;

pub struct ScheduledDecisionRequest {
    pub decision_result_tx: mpsc::UnboundedSender<DecisionResult>,
    pub decision_stop_rx: mpsc::UnboundedReceiver<()>,
    pub plan: DecisionPlan,
    pub slot_guard: SlotGuard,
    pub started_at: Instant,
}

impl SchedulerCommand for ScheduledDecisionRequest {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        DecisionError::SchedulerUnavailable.report(agent_name, &self.decision_result_tx);
    }
}
