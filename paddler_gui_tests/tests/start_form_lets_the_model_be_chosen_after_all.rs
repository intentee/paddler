use iced_test::simulator;

use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::start_balancer_form_message::StartBalancerFormMessage;

#[test]
fn start_form_lets_the_model_be_chosen_after_all() {
    let start_form_data = StartBalancerFormData {
        add_model_later: true,
        ..StartBalancerFormData::suggesting_addresses_on(None)
    };
    let mut start_form = simulator(start_form_data.view());

    start_form
        .click("Add a model later")
        .expect("the start form must let the model be chosen after all");

    assert!(matches!(
        start_form.into_messages().collect::<Vec<_>>().as_slice(),
        [StartBalancerFormMessage::ToggleAddModelLater(false)]
    ));
}
