#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::mem::discriminant;
use std::num::NonZeroU16;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_cluster_harness::decision_done_discriminant::decision_done_discriminant;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::many_question_decision::many_question_decision;

const CONCURRENT_DECISIONS: u16 = 2;

fn assert_decided(mut decision_result_rx: UnboundedReceiver<DecisionResult>) {
    assert!(matches!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::QuestionAnswered(AnsweredDecisionQuestion { id, .. }))
            if id == "paid-0"
    ));
    assert_eq!(
        decision_result_rx
            .try_recv()
            .map(|decision_result| discriminant(&decision_result)),
        Ok(decision_done_discriminant())
    );
}

#[test]
fn a_decision_arriving_while_another_is_decided_is_decided_too() {
    let decision_model_runtime = DecisionModelRuntime::load(&kev_0_8b(), CONCURRENT_DECISIONS)
        .expect("the kev decision model must load");
    let prepared_decision = || {
        decision_model_runtime
            .prepared_decision(many_question_decision(1), NonZeroU16::MIN, u32::MAX)
            .expect("the decision must be prepared")
    };
    let first_decision = prepared_decision();
    let second_decision = prepared_decision();
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for scheduled_decision_request in [first_decision.request, second_decision.request] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(scheduled_decision_request))
            .expect("the scheduler must receive the decision");
    }

    drop(scheduler_message_tx);

    decision_model_runtime
        .run_scheduler(DecisionSchedulerRun {
            active_requests: VecDeque::new(),
            agent_shutdown: CancellationToken::new(),
            capacity_ledger: DecisionCapacityLedger::new(CONCURRENT_DECISIONS),
            pending_requests: VecDeque::new(),
            scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
            sequences: CONCURRENT_DECISIONS,
        })
        .expect("the decision context must be created");

    assert_decided(first_decision.decision_result_rx);
    assert_decided(second_decision.decision_result_rx);
}
