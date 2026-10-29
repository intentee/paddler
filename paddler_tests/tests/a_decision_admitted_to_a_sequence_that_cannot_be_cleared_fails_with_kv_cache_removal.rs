#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;

use llama_cpp_bindings::error::ClearKvCacheSeqError;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_tests::prepared_decision::PreparedDecision;

use crate::kev_decision_model::KevDecisionModel;

const SEQUENCES_THE_CONTEXT_HOLDS: u16 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 2;

#[test]
fn a_decision_admitted_to_a_sequence_that_cannot_be_cleared_fails_with_kv_cache_removal() {
    let kev_decision_model = KevDecisionModel::load(SEQUENCES_THE_LEDGER_HANDS_OUT);
    let prepared_decision = |question_id: &str| {
        PreparedDecision::new(
            kev_decision_model
                .runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
            kev_decision_model.two_option_question_layout(question_id),
        )
    };
    let decision_on_a_clearable_sequence = prepared_decision("clearable");
    let mut decision_on_a_sequence_beyond_the_context = prepared_decision("beyond");
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for prepared_decision_request in [
        decision_on_a_clearable_sequence.prepared_decision_request,
        decision_on_a_sequence_beyond_the_context.prepared_decision_request,
    ] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(prepared_decision_request))
            .expect("the scheduler must receive the decision");
    }

    drop(scheduler_message_tx);

    kev_decision_model.run_scheduler(DecisionSchedulerRun {
        active_requests: VecDeque::new(),
        agent_shutdown: CancellationToken::new(),
        capacity_ledger: DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT),
        pending_requests: VecDeque::new(),
        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
        sequences: SEQUENCES_THE_CONTEXT_HOLDS,
    });

    assert_eq!(
        decision_on_a_sequence_beyond_the_context
            .decision_result_rx
            .try_recv(),
        Ok(DecisionResult::KvCacheRemovalFailed(
            DecisionError::KvCacheRemovalFailed(ClearKvCacheSeqError::PartialSequenceNotRemoved {
                seq_id: 1,
                p0: -1,
                p1: -1,
            })
            .to_string()
        ))
    );
}
