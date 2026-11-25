use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;

use crate::cluster_harness_error::ClusterHarnessError;

pub enum AgentReadiness {
    Registered,
    SlotsReady(u16),
}

impl AgentReadiness {
    #[must_use]
    pub const fn of_registration(slot_count: u16, wait_for_slots_ready: bool) -> Self {
        if wait_for_slots_ready {
            Self::SlotsReady(slot_count)
        } else {
            Self::Registered
        }
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::AgentReportedIssues`] when the agent awaits its slots while it reports issues.
    pub fn is_met_by(
        &self,
        agent_name: &str,
        AgentControllerSnapshot { status, .. }: &AgentControllerSnapshot,
    ) -> Result<bool, ClusterHarnessError> {
        match self {
            Self::Registered => Ok(true),
            Self::SlotsReady(expected_slot_count)
                if status.runtime.slots_total() == u64::from(*expected_slot_count) =>
            {
                Ok(true)
            }
            Self::SlotsReady(_) if !status.issues.is_empty() => {
                Err(ClusterHarnessError::AgentReportedIssues {
                    agent_name: agent_name.to_owned(),
                    issues: status.issues.clone(),
                })
            }
            Self::SlotsReady(_) => Ok(false),
        }
    }
}
