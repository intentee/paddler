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

    pub fn is_met_by(
        &self,
        agent_name: &str,
        registered_agent: &AgentControllerSnapshot,
    ) -> Result<bool, ClusterHarnessError> {
        match self {
            Self::Registered => Ok(true),
            Self::SlotsReady(_) if !registered_agent.status.issues.is_empty() => {
                Err(ClusterHarnessError::AgentReportedIssues {
                    agent_name: agent_name.to_owned(),
                    issues: registered_agent.status.issues.clone(),
                })
            }
            Self::SlotsReady(expected_slot_count) => Ok(registered_agent
                .status
                .runtime
                .slots_total()
                == u64::from(*expected_slot_count)),
        }
    }
}
