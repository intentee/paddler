use iced_test::simulator;

use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::start_balancer_form_message::StartBalancerFormMessage;

#[test]
fn start_form_offers_to_add_the_model_later() {
    let start_form_data = StartBalancerFormData::suggesting_addresses_on(None);
    let mut start_form = simulator(start_form_data.view());

    start_form
        .click("Add a model later")
        .expect("the start form must offer to add the model later");

    assert!(matches!(
        start_form.into_messages().collect::<Vec<_>>().as_slice(),
        [StartBalancerFormMessage::ToggleAddModelLater(true)]
    ));
}
