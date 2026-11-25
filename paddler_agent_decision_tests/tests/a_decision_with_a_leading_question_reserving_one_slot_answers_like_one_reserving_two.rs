#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::num::NonZeroU16;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::sample_decision::sample_decision;

fn answers_of(
    mut decision_result_rx: UnboundedReceiver<DecisionResult>,
) -> Vec<AnsweredDecisionQuestion> {
    let mut answers = Vec::new();

    while let Ok(decision_result) = decision_result_rx.try_recv() {
        if let DecisionResult::QuestionAnswered(answered_question) = decision_result {
            answers.push(answered_question);
        }
    }

    answers
}

#[test]
fn a_decision_with_a_leading_question_reserving_one_slot_answers_like_one_reserving_two() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), DECISION_SLOTS_MINIMUM.get())
            .expect("the kev decision model must load");
    let decision_reserving_two_slots = decision_model_runtime
        .prepared_decision(sample_decision(), DECISION_SLOTS_MINIMUM, u32::MAX)
        .expect("the decision reserving two slots must be prepared");
    let decision_reserving_one_slot = decision_model_runtime
        .prepared_decision(sample_decision(), NonZeroU16::MIN, u32::MAX)
        .expect("the decision reserving one slot must be prepared");
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for scheduled_decision_request in [
        decision_reserving_two_slots.request,
        decision_reserving_one_slot.request,
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
            capacity_ledger: DecisionCapacityLedger::new(DECISION_SLOTS_MINIMUM.get()),
            pending_requests: VecDeque::new(),
            scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
            sequences: DECISION_SLOTS_MINIMUM.get(),
        })
        .expect("the decision context must be created");

    assert_eq!(
        answers_of(decision_reserving_one_slot.decision_result_rx),
        answers_of(decision_reserving_two_slots.decision_result_rx)
    );
}
