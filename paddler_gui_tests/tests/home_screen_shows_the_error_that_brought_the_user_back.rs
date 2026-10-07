use std::sync::Arc;

use iced_test::simulator;

use paddler_gui::home_data::HomeData;
use paddler_gui::runner_failure::RunnerFailure;
use paddler_service_thread::service_thread_error::ServiceThreadError;

#[test]
fn home_screen_shows_the_error_that_brought_the_user_back() {
    let home_data = HomeData::ReturnedAfterFailure(RunnerFailure::Agent(Arc::new(
        ServiceThreadError::ServiceThreadPanicked,
    )));
    let mut home_screen = simulator(home_data.view());

    home_screen
        .find("A Paddler service thread panicked")
        .expect("the home screen must show the error that brought the user back");
}
