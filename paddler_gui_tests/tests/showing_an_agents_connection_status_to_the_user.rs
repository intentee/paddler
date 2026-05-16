use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::agent_running_data::AgentRunningData;
use paddler_gui::ui::view_agent_running::view_agent_running;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

fn data_with_connected(connected: bool) -> AgentRunningData {
    AgentRunningData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        connected,
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
fn the_status_text_says_connected_when_the_agent_has_joined_the_cluster() -> Result<()> {
    let data = data_with_connected(true);
    let mut simulator = simulator(view_agent_running(&data));

    if simulator
        .find("Connected to the cluster at 127.0.0.1:8060")
        .is_err()
    {
        bail!("expected connected status to render with the balancer address");
    }
    Ok(())
}

#[test]
fn the_status_text_says_connecting_before_the_agent_has_joined() -> Result<()> {
    let data = data_with_connected(false);
    let mut simulator = simulator(view_agent_running(&data));

    if simulator.find("Connecting to the cluster...").is_err() {
        bail!("expected connecting status to render before connection completes");
    }
    Ok(())
}
