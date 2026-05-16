use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::agent_running_data::AgentRunningData;
use paddler_gui::agent_running_handler::Message;
use paddler_gui::ui::view_agent_running::view_agent_running;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

fn fresh_agent_running_data() -> AgentRunningData {
    AgentRunningData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        connected: false,
        snapshot: AgentControllerSnapshot {
            desired_slots_total: 0,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: String::new(),
            issues: BTreeSet::new(),
            model_path: None,
            name: None,
            slots_processing: 0,
            slots_total: 0,
            state_application_status: AgentStateApplicationStatus::Fresh,
            uses_chat_template_override: false,
        },
    }
}

#[test]
fn clicking_disconnect_sends_the_disconnect_message() -> Result<()> {
    let data = fresh_agent_running_data();
    let mut simulator = simulator(view_agent_running(&data));

    simulator.click("Disconnect")?;

    match simulator.into_messages().next() {
        Some(Message::Disconnect) => Ok(()),
        other => bail!("expected Disconnect message, got {other:?}"),
    }
}
