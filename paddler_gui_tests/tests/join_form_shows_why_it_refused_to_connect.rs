use iced_test::simulator;

use paddler_gui::join_balancer_form_data::JoinBalancerFormData;
use paddler_gui::join_balancer_form_message::JoinBalancerFormMessage;

#[test]
fn join_form_shows_why_it_refused_to_connect() {
    let mut join_form_data = JoinBalancerFormData::default();
    let _refused_connection = join_form_data.update(JoinBalancerFormMessage::Connect);
    let mut join_form = simulator(join_form_data.view());

    join_form
        .find("Cluster address is required.")
        .expect("the join form must explain the missing cluster address");
    join_form
        .find("Number of slots is required.")
        .expect("the join form must explain the missing slot count");
}
