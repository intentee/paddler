#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::mem::discriminant;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_tests::prepared_decision::PreparedDecision;

use crate::kev_decision_model::KevDecisionModel;

const CONCURRENT_DECISIONS: u16 = 2;

#[test]
fn a_decision_arriving_while_another_is_decided_is_decided_too() {
    let kev_decision_model = KevDecisionModel::load(CONCURRENT_DECISIONS);
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
    let first_decision = prepared_decision("first");
    let second_decision = prepared_decision("second");
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for prepared_decision_request in [
        first_decision.prepared_decision_request,
        second_decision.prepared_decision_request,
    ] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(prepared_decision_request))
            .expect("the scheduler must receive the decision");
    }

    drop(scheduler_message_tx);

    kev_decision_model.run_scheduler(DecisionSchedulerRun {
        active_requests: VecDeque::new(),
        agent_shutdown: CancellationToken::new(),
        capacity_ledger: DecisionCapacityLedger::new(CONCURRENT_DECISIONS),
        pending_requests: VecDeque::new(),
        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
        sequences: CONCURRENT_DECISIONS,
    });

    let assert_decided =
        |question_id: &str, mut decision_result_rx: UnboundedReceiver<DecisionResult>| {
            assert!(matches!(
                decision_result_rx.try_recv(),
                Ok(DecisionResult::QuestionAnswered(DecisionAnswer { id, probabilities }))
                    if id == question_id && probabilities.len() == 2
            ));
            assert_eq!(
                decision_result_rx
                    .try_recv()
                    .map(|decision_result| discriminant(&decision_result)),
                Ok(decision_done_discriminant())
            );
        };

    assert_decided("first", first_decision.decision_result_rx);
    assert_decided("second", second_decision.decision_result_rx);
}
