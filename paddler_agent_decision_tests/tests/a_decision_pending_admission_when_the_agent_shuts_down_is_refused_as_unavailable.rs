#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;
use std::num::NonZeroU16;

use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::decision_scheduler_run::DecisionSchedulerRun;
use paddler_agent_decision_tests::prepared_decision::PreparedDecision;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::many_question_decision::many_question_decision;

const DECISION_SEQUENCES: u16 = 2;

#[test]
fn a_decision_pending_admission_when_the_agent_shuts_down_is_refused_as_unavailable() {
    let decision_model_runtime = DecisionModelRuntime::load(&kev_0_8b(), DECISION_SEQUENCES)
        .expect("the kev decision model must load");
    let PreparedDecision {
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
        request: scheduled_decision_request,
    } = decision_model_runtime
        .prepared_decision(many_question_decision(1), NonZeroU16::MIN, u32::MAX)
        .expect("the decision must be prepared");
    let agent_shutdown = CancellationToken::new();
    let (_scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    agent_shutdown.cancel();

    decision_model_runtime
        .run_scheduler(DecisionSchedulerRun {
            active_requests: VecDeque::new(),
            agent_shutdown,
            capacity_ledger: DecisionCapacityLedger::new(DECISION_SEQUENCES),
            pending_requests: VecDeque::from([scheduled_decision_request]),
            scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
            sequences: DECISION_SEQUENCES,
        })
        .expect("the decision context must be created");

    assert!(matches!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::SchedulerUnavailable(_))
    ));
}
