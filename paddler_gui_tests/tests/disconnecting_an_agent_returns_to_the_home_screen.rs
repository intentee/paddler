use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn disconnecting_an_agent_returns_to_the_home_screen() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let _agent_task = app
        .join_cluster("127.0.0.1:8060".to_owned())
        .expect("the join form must offer to connect");

    let _disconnect_task = app
        .click("Disconnect")
        .expect("the running agent must offer to disconnect");

    app.find("Join a cluster")
        .expect("disconnecting must return to the home screen");
}
