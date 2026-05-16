use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_handler::Message;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui::ui::view_running_balancer::view_running_balancer;

fn data_with_stopping(stopping: bool) -> RunningBalancerData {
    RunningBalancerData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        snapshot: RunningBalancerSnapshot::default(),
        stopping,
        web_admin_panel_address: None,
    }
}

#[test]
fn clicking_stop_cluster_sends_the_stop_message_when_the_balancer_is_running() -> Result<()> {
    let data = data_with_stopping(false);
    let mut simulator = simulator(view_running_balancer(&data));

    simulator.click("Stop cluster")?;

    match simulator.into_messages().next() {
        Some(Message::Stop) => Ok(()),
        other => bail!("expected Stop message, got {other:?}"),
    }
}

#[test]
fn the_stop_button_does_not_react_to_clicks_while_the_balancer_is_already_stopping() -> Result<()> {
    let data = data_with_stopping(true);
    let mut simulator = simulator(view_running_balancer(&data));

    // The inert button has text "Stopping...".
    simulator.click("Stopping...")?;

    if simulator.into_messages().next().is_some() {
        bail!("expected no message when the inert Stopping button is clicked");
    }

    Ok(())
}
