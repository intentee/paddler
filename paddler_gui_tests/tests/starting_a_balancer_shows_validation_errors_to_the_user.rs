use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::address_field::AddressField;
use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::ui::view_start_balancer_form::view_start_balancer_form;

#[test]
fn each_address_field_renders_its_own_error_below_the_input_when_set() -> Result<()> {
    let data = StartBalancerFormData {
        add_model_later: false,
        balancer_address: AddressField::Invalid {
            raw: String::new(),
            error: "Address is required.".to_owned(),
        },
        inference_address: AddressField::Invalid {
            raw: String::new(),
            error: "Invalid inference address.".to_owned(),
        },
        model_error: Some("Please select a model.".to_owned()),
        selected_model: None,
        starting: false,
        web_admin_panel_address: AddressField::Invalid {
            raw: String::new(),
            error: "Invalid web admin address.".to_owned(),
        },
        web_admin_panel_address_placeholder: String::new(),
    };
    let mut simulator = simulator(view_start_balancer_form(&data));

    for expected in [
        "Address is required.",
        "Invalid inference address.",
        "Please select a model.",
        "Invalid web admin address.",
    ] {
        if simulator.find(expected).is_err() {
            bail!("expected to find error text {expected:?} in the rendered tree");
        }
    }

    Ok(())
}
