use std::sync::Arc;

use iced_test::simulator;

use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_gui::home_data::HomeData;

#[test]
fn home_screen_shows_the_error_that_brought_the_user_back() {
    let home_data = HomeData {
        error: Some(Arc::new(BootstrapError::ServiceThreadPanicked)),
    };
    let mut home_screen = simulator(home_data.view());

    home_screen
        .find("A Paddler service thread panicked")
        .expect("the home screen must show the error that brought the user back");
}
