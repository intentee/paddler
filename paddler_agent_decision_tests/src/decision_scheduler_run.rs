use std::collections::VecDeque;

use tokio_util::sync::CancellationToken;

use paddler_agent_decision::active_decision_request::ActiveDecisionRequest;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::scheduled_decision_request::ScheduledDecisionRequest;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;

pub struct DecisionSchedulerRun {
    pub active_requests: VecDeque<ActiveDecisionRequest>,
    pub agent_shutdown: CancellationToken,
    pub capacity_ledger: DecisionCapacityLedger,
    pub pending_requests: VecDeque<ScheduledDecisionRequest>,
    pub scheduler_inbox: SchedulerInbox<ScheduledDecisionRequest>,
    pub sequences: u16,
}
