use crate::slots_insufficient_for_decisions::slots_insufficient_for_decisions;
use crate::startup_superseded_while_its_blocking_work_waits::StartupSupersededWhileItsBlockingWorkWaits;
use crate::state_enabling_decisions::state_enabling_decisions;

#[test]
fn a_state_superseding_a_decisions_preparation_that_fails_leaves_no_issue_behind() {
    let slot_aggregated_status = StartupSupersededWhileItsBlockingWorkWaits {
        superseded_state: state_enabling_decisions(),
    }
    .run();

    assert!(
        !slot_aggregated_status.has_issue(&slots_insufficient_for_decisions(
            slot_aggregated_status.desired_slots_total
        ))
    );
}
