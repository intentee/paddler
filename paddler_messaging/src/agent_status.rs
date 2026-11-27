use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue::AgentIssue;
use crate::agent_runtime_status::AgentRuntimeStatus;
use crate::agent_state_application_status::AgentStateApplicationStatus;
use crate::desired_state_version::DesiredStateVersion;
use crate::model_download_status::ModelDownloadStatus;
use crate::requested_capability::RequestedCapability;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStatus {
    pub desired_slots_total: u16,
    pub desired_state_version: DesiredStateVersion,
    pub download_status: ModelDownloadStatus,
    pub issues: BTreeSet<AgentIssue>,
    pub model_path: Option<String>,
    pub runtime: AgentRuntimeStatus,
    pub state_application_status: AgentStateApplicationStatus,
    pub uses_chat_template_override: bool,
}

impl AgentStatus {
    #[must_use]
    pub const fn may_serve(&self, requested_capability: RequestedCapability) -> bool {
        self.runtime.serves(requested_capability) || self.is_applying_its_desired_state()
    }

    const fn is_applying_its_desired_state(&self) -> bool {
        match self.download_status {
            ModelDownloadStatus::Downloading { .. }
            | ModelDownloadStatus::DownloadingWithUnknownSize { .. } => true,
            ModelDownloadStatus::NotDownloading => match self.state_application_status {
                AgentStateApplicationStatus::Applied | AgentStateApplicationStatus::Stuck => false,
                AgentStateApplicationStatus::AttemptedAndRetrying
                | AgentStateApplicationStatus::Fresh => true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_model_capabilities_fixture::decision_model_capabilities::decision_model_capabilities;

    use super::AgentStatus;
    use crate::agent_runtime_status::AgentRuntimeStatus;
    use crate::agent_state_application_status::AgentStateApplicationStatus;
    use crate::model_download_status::ModelDownloadStatus;
    use crate::requested_capability::RequestedCapability;

    fn settled_agent(state_application_status: AgentStateApplicationStatus) -> AgentStatus {
        AgentStatus {
            state_application_status,
            ..AgentStatus::default()
        }
    }

    #[test]
    fn a_settled_agent_may_serve_only_what_it_serves() {
        let serving_decisions = AgentStatus {
            runtime: AgentRuntimeStatus::Serving {
                capabilities: decision_model_capabilities(),
                slots_total: 2,
            },
            ..settled_agent(AgentStateApplicationStatus::Applied)
        };

        assert!(serving_decisions.may_serve(RequestedCapability::Decisions));
        assert!(!serving_decisions.may_serve(RequestedCapability::Embeddings));
    }

    #[test]
    fn an_agent_that_gave_up_applying_its_state_may_serve_nothing_it_does_not_serve() {
        assert!(
            !settled_agent(AgentStateApplicationStatus::Stuck)
                .may_serve(RequestedCapability::Decisions)
        );
    }

    #[test]
    fn an_agent_still_applying_its_state_may_come_to_serve_any_capability() {
        for state_application_status in [
            AgentStateApplicationStatus::AttemptedAndRetrying,
            AgentStateApplicationStatus::Fresh,
        ] {
            assert!(
                settled_agent(state_application_status).may_serve(RequestedCapability::Decisions)
            );
        }
    }

    #[test]
    fn an_agent_downloading_a_model_may_come_to_serve_any_capability() {
        assert!(
            AgentStatus {
                download_status: ModelDownloadStatus::DownloadingWithUnknownSize {
                    downloaded_bytes: 0,
                    model_path: "model.gguf".to_owned(),
                },
                ..settled_agent(AgentStateApplicationStatus::Applied)
            }
            .may_serve(RequestedCapability::Decisions)
        );
    }
}
