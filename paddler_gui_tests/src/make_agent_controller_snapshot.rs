use std::collections::BTreeSet;

use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

pub struct AgentSnapshotFixture {
    pub desired_slots_total: i32,
    pub download_current: usize,
    pub download_filename: Option<String>,
    pub download_total: usize,
    pub id: String,
    pub model_path: Option<String>,
    pub name: Option<String>,
    pub slots_processing: i32,
    pub slots_total: i32,
    pub state_application_status: AgentStateApplicationStatus,
}

impl Default for AgentSnapshotFixture {
    fn default() -> Self {
        Self {
            desired_slots_total: 4,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: "fixture-agent-id".to_owned(),
            model_path: None,
            name: None,
            slots_processing: 0,
            slots_total: 4,
            state_application_status: AgentStateApplicationStatus::Fresh,
        }
    }
}

pub fn make_agent_controller_snapshot(fixture: AgentSnapshotFixture) -> AgentControllerSnapshot {
    AgentControllerSnapshot {
        desired_slots_total: fixture.desired_slots_total,
        download_current: fixture.download_current,
        download_filename: fixture.download_filename,
        download_total: fixture.download_total,
        id: fixture.id,
        issues: BTreeSet::new(),
        model_path: fixture.model_path,
        name: fixture.name,
        slots_processing: fixture.slots_processing,
        slots_total: fixture.slots_total,
        state_application_status: fixture.state_application_status,
        uses_chat_template_override: false,
    }
}
