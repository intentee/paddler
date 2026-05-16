use anyhow::Result;
use anyhow::bail;
use iced::widget::text_input;
use iced_test::simulator;
use paddler_gui::message::Message;
use paddler_gui::ui::view_form_field::view_form_field;

#[test]
fn a_form_field_without_an_error_renders_only_label_and_input() -> Result<()> {
    let input = text_input("placeholder", "value").into();
    let mut simulator = simulator(view_form_field::<Message>("Cluster address", input, None));

    if simulator.find("Cluster address").is_err() {
        bail!("expected the form field label to render");
    }

    Ok(())
}

#[test]
fn a_form_field_with_an_error_renders_the_error_text_below_the_input() -> Result<()> {
    let input = text_input("placeholder", "value").into();
    let error = "Address is required.".to_owned();
    let mut simulator = simulator(view_form_field::<Message>(
        "Cluster address",
        input,
        Some(&error),
    ));

    if simulator.find("Cluster address").is_err() {
        bail!("expected the label to render");
    }
    if simulator.find("Address is required.").is_err() {
        bail!("expected the error text to render");
    }

    Ok(())
}
