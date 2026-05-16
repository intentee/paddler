use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_handler::Message;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui::ui::view_running_balancer::view_running_balancer;

#[test]
fn clicking_copy_address_sends_the_copy_to_clipboard_message_with_the_balancer_address()
-> Result<()> {
    let data = RunningBalancerData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        snapshot: RunningBalancerSnapshot::default(),
        stopping: false,
        web_admin_panel_address: None,
    };
    let mut simulator = simulator(view_running_balancer(&data));

    simulator.click("Copy address")?;

    match simulator.into_messages().next() {
        Some(Message::CopyToClipboard(addr)) if addr == "127.0.0.1:8060" => Ok(()),
        other => bail!("expected CopyToClipboard(127.0.0.1:8060), got {other:?}"),
    }
}
