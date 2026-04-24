use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

pub struct AgentsStatus;

impl AgentsStatus {
    pub fn agent_count_at_least(
        expected_count: usize,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        move |snapshot| snapshot.agents.len() >= expected_count
    }

    pub fn agent_count_is(expected_count: usize) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        move |snapshot| snapshot.agents.len() == expected_count
    }

    pub fn agent_registered(agent_id: &str) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| snapshot.agents.iter().any(|agent| agent.id == agent_id)
    }

    pub fn download_finished(agent_id: &str) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.download_filename.is_none())
        }
    }

    pub fn download_progressed(agent_id: &str) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.download_current > 0)
        }
    }

    pub fn has_issue(
        agent_id: &str,
        issue: AgentIssue,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.issues.contains(&issue))
        }
    }

    pub fn has_any_issue(agent_id: &str) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && !agent.issues.is_empty())
        }
    }

    pub fn slots_processing_is(
        agent_id: &str,
        expected_slots_processing: i32,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id && agent.slots_processing == expected_slots_processing
            })
        }
    }

    pub fn slots_total_at_least(
        agent_id: &str,
        expected_slots_total: i32,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.slots_total >= expected_slots_total)
        }
    }

    pub fn state_application_status_applied(
        agent_id: &str,
    ) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
        let agent_id = agent_id.to_owned();

        move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent.state_application_status == AgentStateApplicationStatus::Applied
            })
        }
    }
}
