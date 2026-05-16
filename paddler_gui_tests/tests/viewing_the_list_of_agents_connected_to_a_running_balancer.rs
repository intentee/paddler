use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui::ui::view_running_balancer::view_running_balancer;
use paddler_gui_tests::make_agent_controller_snapshot::AgentSnapshotFixture;
use paddler_gui_tests::make_agent_controller_snapshot::make_agent_controller_snapshot;

#[test]
fn when_no_agents_are_connected_the_screen_shows_a_waiting_message() -> Result<()> {
    let data = RunningBalancerData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        snapshot: RunningBalancerSnapshot::default(),
        stopping: false,
        web_admin_panel_address: None,
    };
    let mut simulator = simulator(view_running_balancer(&data));

    if simulator.find("Waiting for agents to connect...").is_err() {
        bail!("expected the waiting message to render when agent list is empty");
    }

    Ok(())
}

#[test]
fn each_connected_agent_appears_as_its_own_card() -> Result<()> {
    let agent = make_agent_controller_snapshot(AgentSnapshotFixture {
        name: Some("primary-agent".to_owned()),
        model_path: Some("/models/model.gguf".to_owned()),
        ..AgentSnapshotFixture::default()
    });

    let snapshot = RunningBalancerSnapshot {
        agent_snapshots: vec![agent],
        ..RunningBalancerSnapshot::default()
    };

    let data = RunningBalancerData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        snapshot,
        stopping: false,
        web_admin_panel_address: None,
    };
    let mut simulator = simulator(view_running_balancer(&data));

    if simulator.find("primary-agent").is_err() {
        bail!("expected the agent card to render the agent name");
    }

    Ok(())
}
