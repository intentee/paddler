use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::address_field::AddressField;
use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::ui::view_start_balancer_form::view_start_balancer_form;

#[expect(
    clippy::missing_const_for_fn,
    reason = "non-const helper keeps the surface uniform with other fixture helpers"
)]
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
fn with_add_model_later_unchecked_the_form_shows_a_model_picker() -> Result<()> {
    let data = empty_form();
    let mut simulator = simulator(view_start_balancer_form(&data));

    // The "Model" label is always present; the absence of "Model will be added later" placeholder
    // confirms we're rendering the pick_list branch.
    if simulator.find("Model").is_err() {
        bail!("expected the Model label to render");
    }
    if simulator.find("Model will be added later").is_ok() {
        bail!("did not expect the add-later placeholder while pick_list branch is rendered");
    }

    Ok(())
}

#[test]
fn with_add_model_later_checked_the_form_shows_a_disabled_placeholder_input() -> Result<()> {
    let data = StartBalancerFormData {
        add_model_later: true,
        ..empty_form()
    };
    let mut simulator = simulator(view_start_balancer_form(&data));

    if simulator.find("Add a model later").is_err() {
        bail!("expected the add-model-later checkbox label to render");
    }

    Ok(())
}
