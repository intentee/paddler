use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::join_balancer_form_data::JoinBalancerFormData;
use paddler_gui::join_balancer_form_handler::Message;
use paddler_gui::ui::view_join_balancer_form::view_join_balancer_form;

#[test]
fn clicking_connect_sends_the_connect_message() -> Result<()> {
    let data = JoinBalancerFormData::default();
    let mut simulator = simulator(view_join_balancer_form(&data));

    simulator.click("Connect")?;

    match simulator.into_messages().next() {
        Some(Message::Connect) => Ok(()),
        other => bail!("expected Connect message, got {other:?}"),
    }
}

#[test]
fn clicking_cancel_sends_the_cancel_message() -> Result<()> {
    let data = JoinBalancerFormData::default();
    let mut simulator = simulator(view_join_balancer_form(&data));

    simulator.click("Cancel")?;

    match simulator.into_messages().next() {
        Some(Message::Cancel) => Ok(()),
        other => bail!("expected Cancel message, got {other:?}"),
    }
}

#[test]
fn agent_name_field_is_findable_for_user_input() -> Result<()> {
    let data = JoinBalancerFormData {
        agent_name: "primary".to_owned(),
        ..JoinBalancerFormData::default()
    };
    let mut simulator = simulator(view_join_balancer_form(&data));

    match simulator.find("primary") {
        Ok(_) => Ok(()),
        Err(error) => bail!("expected the populated agent name to render: {error}"),
    }
}
