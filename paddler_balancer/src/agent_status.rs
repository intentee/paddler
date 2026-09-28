use std::collections::BTreeSet;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller_update_result::AgentControllerUpdateResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentStatus {
    pub desired_slots_total: i32,
    pub download_current: u64,
    pub download_filename: Option<String>,
    pub download_indeterminate: bool,
    pub download_total: u64,
    pub issues: BTreeSet<AgentIssue>,
    pub model_path: Option<String>,
    pub slots_total: i32,
    pub state_application_status: AgentStateApplicationStatus,
    pub uses_chat_template_override: bool,
    pub version: i32,
}

impl AgentStatus {
    pub fn absorb(&mut self, reported_status: Self) -> AgentControllerUpdateResult {
        if reported_status.version < self.version {
            return AgentControllerUpdateResult::NoMeaningfulChanges;
        }

        self.version = reported_status.version;

        if *self == reported_status {
            AgentControllerUpdateResult::NoMeaningfulChanges
        } else {
            *self = reported_status;

            AgentControllerUpdateResult::Updated
        }
    }
}

impl From<SlotAggregatedStatusSnapshot> for AgentStatus {
    fn from(
        SlotAggregatedStatusSnapshot {
            desired_slots_total,
            download_current,
            download_filename,
            download_indeterminate,
            download_total,
            issues,
            model_path,
            slots_total,
            state_application_status,
            uses_chat_template_override,
            version,
            ..
        }: SlotAggregatedStatusSnapshot,
    ) -> Self {
        Self {
            desired_slots_total,
            download_current,
            download_filename,
            download_indeterminate,
            download_total,
            issues,
            model_path,
            slots_total,
            state_application_status,
            uses_chat_template_override,
            version,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentStatus;
    use crate::agent_controller_update_result::AgentControllerUpdateResult;

    fn reported(version: i32, slots_total: i32) -> AgentStatus {
        AgentStatus::from(SlotAggregatedStatusSnapshot {
            slots_total,
            version,
            ..SlotAggregatedStatusSnapshot::default()
        })
    }

    #[test]
    fn keeps_the_newer_status_when_an_older_report_arrives_late() {
        let mut status = reported(2, 4);

        assert!(matches!(
            status.absorb(reported(1, 8)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        ));
        assert_eq!(status, reported(2, 4));
    }

    #[test]
    fn reports_no_change_for_a_newer_report_with_the_same_values() {
        let mut status = reported(1, 4);

        assert!(matches!(
            status.absorb(reported(2, 4)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        ));
        assert_eq!(status.version, 2);
    }

    #[test]
    fn takes_a_newer_report_with_changed_values() {
        let mut status = reported(1, 4);

        assert!(matches!(
            status.absorb(reported(2, 8)),
            AgentControllerUpdateResult::Updated
        ));
        assert_eq!(status, reported(2, 8));
    }
}
