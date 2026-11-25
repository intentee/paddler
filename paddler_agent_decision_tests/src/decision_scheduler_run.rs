use std::collections::VecDeque;

use tokio_util::sync::CancellationToken;

use paddler_agent_decision::active_decision_request::ActiveDecisionRequest;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::prepared_decision_request::PreparedDecisionRequest;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;

pub struct DecisionSchedulerRun {
    pub active_requests: VecDeque<ActiveDecisionRequest>,
    pub agent_shutdown: CancellationToken,
    pub capacity_ledger: DecisionCapacityLedger,
    pub pending_requests: VecDeque<PreparedDecisionRequest>,
    pub scheduler_inbox: SchedulerInbox<PreparedDecisionRequest>,
    pub sequences: u16,
}
