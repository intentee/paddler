use std::net::TcpListener;

use iced_test::core::Size;
use iced_test::program::Program;

use paddler_gui::message::Message;
use paddler_gui::paddler_application::paddler_application;
use paddler_gui_tests::emulated_app::EmulatedApp;
use paddler_gui_tests::gui_test_error::GuiTestError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

fn press_tab<TProgram>(app: &mut EmulatedApp<TProgram>, shift: bool) -> Result<(), GuiTestError>
where
    TProgram: Program<Message = Message> + 'static,
{
    app.update(Message::TabPressed { shift });
    app.perform_next_action()
}

#[test]
fn the_join_form_can_be_filled_with_the_keyboard() {
    let unresponsive_balancer =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("the unresponsive balancer must bind");
    let balancer_address = unresponsive_balancer
        .local_addr()
        .expect("the unresponsive balancer must report its address")
        .to_string();
    let mut app = EmulatedApp::boot(paddler_application(), Size::new(800.0, 800.0))
        .expect("the app must boot");

    app.click("Join a cluster")
        .expect("the home screen must offer to join a cluster");
    press_tab(&mut app, false).expect("tab must focus the cluster address");
    app.typewrite(&balancer_address)
        .expect("the cluster address must accept typing");
    press_tab(&mut app, false).expect("tab must focus the agent name");
    press_tab(&mut app, false).expect("tab must focus the slots");
    press_tab(&mut app, true).expect("shift tab must focus the agent name again");
    app.typewrite("gpu-box")
        .expect("the agent name must accept typing");
    press_tab(&mut app, false).expect("tab must focus the slots again");
    app.typewrite("2").expect("the slots must accept typing");
    app.click("Connect")
        .expect("the join form must offer to connect");
    app.expect_text("Connecting to the cluster...")
        .expect("the filled form must connect the agent");
    app.expect_text("gpu-box")
        .expect("the connecting agent must carry the typed name");
}
