use paddler_agent::reconciliation_progress::ReconciliationProgress;
use paddler_agent_tests::reconciliation_after_replacement::ReconciliationAfterReplacement;
use paddler_agent_tests::reconciliation_replaced_before_its_worker_runs::ReconciliationReplacedBeforeItsWorkerRuns;
use paddler_messaging::agent_desired_state::AgentDesiredState;

#[test]
fn a_reconciliation_replaced_before_its_worker_runs_never_publishes_its_state() {
    let ReconciliationAfterReplacement {
        reconciled_state_holder,
        ..
    } = ReconciliationReplacedBeforeItsWorkerRuns {
        replaced_desired_state: AgentDesiredState::default(),
    }
    .run()
    .expect("the replaced reconciliation must run to its end");

    assert_eq!(
        reconciled_state_holder.get_reconciliation_progress(),
        ReconciliationProgress::NothingReconciled
    );
}
