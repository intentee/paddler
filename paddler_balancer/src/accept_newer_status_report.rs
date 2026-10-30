use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller_update_result::AgentControllerUpdateResult;

pub fn accept_newer_status_report(
    current_snapshot: &mut SlotAggregatedStatusSnapshot,
    reported_snapshot: SlotAggregatedStatusSnapshot,
) -> AgentControllerUpdateResult {
    if reported_snapshot.version < current_snapshot.version {
        return AgentControllerUpdateResult::NoMeaningfulChanges;
    }

    let status_changed = reported_snapshot.status != current_snapshot.status;

    *current_snapshot = reported_snapshot;

    if status_changed {
        AgentControllerUpdateResult::Updated
    } else {
        AgentControllerUpdateResult::NoMeaningfulChanges
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::accept_newer_status_report;
    use crate::agent_controller_update_result::AgentControllerUpdateResult;

    fn reported(version: u64, desired_slots_total: u16) -> SlotAggregatedStatusSnapshot {
        SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                desired_slots_total,
                ..AgentStatus::default()
            },
            version,
        }
    }

    #[test]
    fn keeps_the_newer_status_when_an_older_report_arrives_late() {
        let mut current_snapshot = reported(2, 4);

        assert_eq!(
            accept_newer_status_report(&mut current_snapshot, reported(1, 8)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        );
        assert_eq!(current_snapshot, reported(2, 4));
    }

    #[test]
    fn reports_no_change_for_a_newer_report_with_the_same_status() {
        let mut current_snapshot = reported(1, 4);

        assert_eq!(
            accept_newer_status_report(&mut current_snapshot, reported(2, 4)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        );
        assert_eq!(current_snapshot.version, 2);
    }

    #[test]
    fn takes_a_newer_report_with_a_changed_status() {
        let mut current_snapshot = reported(1, 4);

        assert_eq!(
            accept_newer_status_report(&mut current_snapshot, reported(2, 8)),
            AgentControllerUpdateResult::Updated
        );
        assert_eq!(current_snapshot, reported(2, 8));
    }
}
