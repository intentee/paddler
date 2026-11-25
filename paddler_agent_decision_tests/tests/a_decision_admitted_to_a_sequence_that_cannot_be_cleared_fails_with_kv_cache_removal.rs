#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::num::NonZeroU16;

use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::many_question_decision::many_question_decision;

const SEQUENCES_THE_CONTEXT_HOLDS: u16 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 2;

#[test]
fn a_decision_admitted_to_a_sequence_that_cannot_be_cleared_fails_with_kv_cache_removal() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), SEQUENCES_THE_LEDGER_HANDS_OUT)
            .expect("the kev decision model must load");
    let prepared_decision = || {
        decision_model_runtime
            .prepared_decision(many_question_decision(1), NonZeroU16::MIN, u32::MAX)
            .expect("the decision must be prepared")
    };
    let decision_on_a_clearable_sequence = prepared_decision();
    let mut decision_on_a_sequence_beyond_the_context = prepared_decision();
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for scheduled_decision_request in [
        decision_on_a_clearable_sequence.request,
        decision_on_a_sequence_beyond_the_context.request,
    ] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(scheduled_decision_request))
            .expect("the scheduler must receive the decision");
    }

    drop(scheduler_message_tx);

    decision_model_runtime
        .run_scheduler(DecisionSchedulerRun {
            active_requests: VecDeque::new(),
            agent_shutdown: CancellationToken::new(),
            capacity_ledger: DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT),
            pending_requests: VecDeque::new(),
            scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
            sequences: SEQUENCES_THE_CONTEXT_HOLDS,
        })
        .expect("the decision context must be created");

    assert!(matches!(
        decision_on_a_sequence_beyond_the_context
            .decision_result_rx
            .try_recv(),
        Ok(DecisionResult::KvCacheRemovalFailed(_))
    ));
}
