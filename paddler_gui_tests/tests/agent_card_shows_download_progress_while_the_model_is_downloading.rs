use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::message::Message;
use paddler_gui::ui::view_agent_card::view_agent_card;
use paddler_gui_tests::make_agent_controller_snapshot::AgentSnapshotFixture;
use paddler_gui_tests::make_agent_controller_snapshot::make_agent_controller_snapshot;

#[test]
fn a_named_agent_in_the_middle_of_downloading_renders_a_progress_bar_and_percentage() -> Result<()>
{
    let snapshot = make_agent_controller_snapshot(AgentSnapshotFixture {
        name: Some("downloading-agent".to_owned()),
        download_current: 25,
        download_total: 100,
        ..AgentSnapshotFixture::default()
    });

    let mut simulator = simulator(view_agent_card::<Message>(&snapshot));

    if simulator.find("downloading-agent").is_err() {
        bail!("expected the agent name to render");
    }
    if simulator.find("Status: Downloading (25%)").is_err() {
        bail!("expected the download-progress status to render");
    }

    Ok(())
}
