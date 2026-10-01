use iced_test::simulator;

use paddler_gui::join_balancer_form_data::JoinBalancerFormData;
use paddler_gui::join_balancer_form_message::JoinBalancerFormMessage;

#[test]
fn join_form_edits_the_agent_name() {
    let join_form_data = JoinBalancerFormData {
        agent_name: "gpu-box".to_owned(),
        ..JoinBalancerFormData::default()
    };
    let mut join_form = simulator(join_form_data.view());

    join_form
        .click("gpu-box")
        .expect("the join form must show the entered agent name");
    join_form.typewrite("2");

    assert!(matches!(
        join_form.into_messages().last(),
        Some(JoinBalancerFormMessage::SetAgentName(agent_name)) if agent_name == "gpu-box2"
    ));
}
