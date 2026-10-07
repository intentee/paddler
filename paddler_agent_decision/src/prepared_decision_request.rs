use std::time::Instant;

use tokio::sync::mpsc;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::decision_result::DecisionResult;

use crate::decision_error::DecisionError;
use crate::decision_token_layout::DecisionTokenLayout;

pub struct PreparedDecisionRequest {
    pub decision_result_tx: mpsc::UnboundedSender<DecisionResult>,
    pub decision_stop_rx: mpsc::UnboundedReceiver<()>,
    pub layout: DecisionTokenLayout,
    pub slot_guard: SlotGuard,
    pub started_at: Instant,
}

impl SchedulerCommand for PreparedDecisionRequest {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        DecisionError::SchedulerUnavailable.report(agent_name, &self.decision_result_tx);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::time::Instant;

    use llama_cpp_bindings::token::LlamaToken;
    use tokio::sync::mpsc;

    use paddler_agent_runtime::scheduler_command::SchedulerCommand as _;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::decision_result::DecisionResult;

    use super::PreparedDecisionRequest;
    use crate::decision_question_row::DecisionQuestionRow;
    use crate::decision_token_layout::DecisionTokenLayout;

    #[test]
    fn rejects_a_decision_when_the_scheduler_stopped() {
        let (decision_result_tx, mut decision_result_rx) = mpsc::unbounded_channel();
        let (_decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();

        PreparedDecisionRequest {
            decision_result_tx,
            decision_stop_rx,
            layout: DecisionTokenLayout {
                first_question: DecisionQuestionRow {
                    id: "paid".to_owned(),
                    option_end_offsets: Vec::new(),
                    tokens: vec![LlamaToken::new(0)],
                },
                remaining_questions: VecDeque::new(),
                state: Vec::new(),
            },
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            started_at: Instant::now(),
        }
        .reject_because_the_scheduler_stopped(Some("agent"));

        assert_eq!(
            decision_result_rx.try_recv(),
            Ok(DecisionResult::SchedulerUnavailable(
                "agent: the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }
}
