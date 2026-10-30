#![cfg(feature = "tests_that_use_llms")]

use std::collections::VecDeque;

use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::decision_scheduler_run::DecisionSchedulerRun;

use crate::kev_decision_model::KevDecisionModel;

const DECISION_SEQUENCES: u16 = 2;

#[test]
fn a_decision_running_when_the_agent_shuts_down_is_refused_as_unavailable() {
    let kev_decision_model = KevDecisionModel::load(DECISION_SEQUENCES);
    let capacity_ledger = DecisionCapacityLedger::new(DECISION_SEQUENCES);
    let layout = kev_decision_model.two_option_question_layout("question");
    let AdmittedDecision {
        active_decision_request,
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &capacity_ledger,
        kev_decision_model
            .runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");
    let agent_shutdown = CancellationToken::new();
    let (_scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    agent_shutdown.cancel();

    kev_decision_model.run_scheduler(DecisionSchedulerRun {
        active_requests: VecDeque::from([active_decision_request]),
        agent_shutdown,
        capacity_ledger,
        pending_requests: VecDeque::new(),
        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
        sequences: DECISION_SEQUENCES,
    });

    assert_eq!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::SchedulerUnavailable(
            DecisionError::SchedulerUnavailable.to_string()
        ))
    );
}
