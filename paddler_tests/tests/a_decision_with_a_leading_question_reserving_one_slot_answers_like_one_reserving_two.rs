#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::num::NonZeroU16;

use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_tests::prepared_decision::PreparedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

fn answers_of(mut decision_result_rx: UnboundedReceiver<DecisionResult>) -> Vec<DecisionAnswer> {
    let mut answers = Vec::new();

    while let Ok(decision_result) = decision_result_rx.try_recv() {
        if let DecisionResult::QuestionAnswered(answer) = decision_result {
            answers.push(answer);
        }
    }

    answers
}

#[test]
fn a_decision_with_a_leading_question_reserving_one_slot_answers_like_one_reserving_two() {
    let kev_decision_model = KevDecisionModel::load(DECISION_SLOTS_MINIMUM.get());
    let slot_aggregated_status = kev_decision_model
        .runtime
        .inference_runtime_context
        .slot_aggregated_status
        .clone();
    let layout_with_a_leading_question = || {
        DecisionTokenLayout::new(
            &kev_decision_model.delimiter_tokens,
            vec![LlamaToken::new(0)],
            vec![two_option_question("leading")],
            two_option_question("last"),
        )
    };
    let decision_reserving_two_slots = PreparedDecision::new(
        slot_aggregated_status.clone(),
        layout_with_a_leading_question(),
    );
    let mut decision_reserving_one_slot = PreparedDecision::new(
        slot_aggregated_status.clone(),
        layout_with_a_leading_question(),
    );

    decision_reserving_one_slot
        .prepared_decision_request
        .slot_guard = SlotGuard::new(slot_aggregated_status, NonZeroU16::MIN);

    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for prepared_decision_request in [
        decision_reserving_two_slots.prepared_decision_request,
        decision_reserving_one_slot.prepared_decision_request,
    ] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(prepared_decision_request))
            .expect("the scheduler must receive the decision");
    }

    drop(scheduler_message_tx);

    kev_decision_model.run_scheduler(DecisionSchedulerRun {
        active_requests: VecDeque::new(),
        agent_shutdown: CancellationToken::new(),
        capacity_ledger: DecisionCapacityLedger::new(DECISION_SLOTS_MINIMUM.get()),
        pending_requests: VecDeque::new(),
        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
        sequences: DECISION_SLOTS_MINIMUM.get(),
    });

    assert_eq!(
        answers_of(decision_reserving_one_slot.decision_result_rx),
        answers_of(decision_reserving_two_slots.decision_result_rx)
    );
}
