use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::address_field::AddressField;
use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::start_balancer_form_handler::Message;
use paddler_gui::ui::view_start_balancer_form::view_start_balancer_form;

fn empty_form() -> StartBalancerFormData {
    StartBalancerFormData {
        add_model_later: false,
        balancer_address: AddressField::Empty,
        inference_address: AddressField::Empty,
        model_error: None,
        selected_model: None,
        starting: false,
        web_admin_panel_address: AddressField::Empty,
        web_admin_panel_address_placeholder: String::new(),
    }
}

#[test]
fn clicking_start_cluster_sends_the_confirm_message_when_the_form_is_idle() -> Result<()> {
    let data = empty_form();
    let mut simulator = simulator(view_start_balancer_form(&data));

    simulator.click("Start cluster")?;

    match simulator.into_messages().next() {
        Some(Message::Confirm) => Ok(()),
        other => bail!("expected Confirm message, got {other:?}"),
    }
}

#[test]
fn the_start_button_does_not_react_to_clicks_while_the_balancer_is_already_starting() -> Result<()>
{
    let data = StartBalancerFormData {
        starting: true,
        ..empty_form()
    };
    let mut simulator = simulator(view_start_balancer_form(&data));

    // The button label changes to "Starting..." and has no on_press.
    let click_result = simulator.click("Starting...");

    if click_result.is_err() {
        bail!("expected a Starting... button to be present in the tree");
    }

    if simulator.into_messages().next().is_some() {
        bail!("expected no message when the inert Starting button is clicked");
    }

    Ok(())
}

#[test]
fn clicking_cancel_sends_the_cancel_message() -> Result<()> {
    let data = empty_form();
    let mut simulator = simulator(view_start_balancer_form(&data));

    simulator.click("Cancel")?;

    match simulator.into_messages().next() {
        Some(Message::Cancel) => Ok(()),
        other => bail!("expected Cancel message, got {other:?}"),
    }
}
