use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue::AgentIssue;
use crate::agent_runtime_status::AgentRuntimeStatus;
use crate::agent_state_application_status::AgentStateApplicationStatus;
use crate::model_download_status::ModelDownloadStatus;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStatus {
    pub desired_slots_total: u16,
    pub download_status: ModelDownloadStatus,
    pub issues: BTreeSet<AgentIssue>,
    pub model_path: Option<String>,
    pub runtime: AgentRuntimeStatus,
    pub state_application_status: AgentStateApplicationStatus,
    pub uses_chat_template_override: bool,
}
