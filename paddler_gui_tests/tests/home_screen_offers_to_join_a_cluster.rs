use iced_test::simulator;

use paddler_gui::home_data::HomeData;
use paddler_gui::home_message::HomeMessage;

#[test]
fn home_screen_offers_to_join_a_cluster() {
    let home_data = HomeData { error: None };
    let mut home_screen = simulator(home_data.view());

    home_screen
        .click("Join a cluster")
        .expect("the home screen must offer to join a cluster");

    assert!(matches!(
        home_screen.into_messages().collect::<Vec<_>>().as_slice(),
        [HomeMessage::JoinBalancer]
    ));
}
