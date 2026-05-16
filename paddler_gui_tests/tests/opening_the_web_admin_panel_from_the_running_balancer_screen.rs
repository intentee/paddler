use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_handler::Message;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui::ui::view_running_balancer::view_running_balancer;

fn data_with_panel(address: Option<&str>) -> RunningBalancerData {
    RunningBalancerData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        snapshot: RunningBalancerSnapshot::default(),
        stopping: false,
        web_admin_panel_address: address.map(str::to_owned),
    }
}

#[test]
fn clicking_open_in_browser_sends_the_open_url_message_when_a_panel_address_is_set() -> Result<()>
{
    let data = data_with_panel(Some("127.0.0.1:8062"));
    let mut simulator = simulator(view_running_balancer(&data));

    simulator.click("Open in browser")?;

    match simulator.into_messages().next() {
        Some(Message::OpenUrl(url)) if url == "http://127.0.0.1:8062" => Ok(()),
        other => bail!("expected OpenUrl(http://127.0.0.1:8062), got {other:?}"),
    }
}

#[test]
fn the_open_in_browser_button_is_hidden_when_no_panel_address_is_set() -> Result<()> {
    let data = data_with_panel(None);
    let mut simulator = simulator(view_running_balancer(&data));

    if simulator.find("Open in browser").is_ok() {
        bail!("expected the open-in-browser button to be absent without a panel address");
    }
    Ok(())
}
