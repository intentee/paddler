use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::home_data::HomeData;
use paddler_gui::ui::view_home::view_home;

#[test]
fn home_displays_the_error_message_that_was_carried_over_from_the_previous_screen() -> Result<()> {
    let data = HomeData {
        error: Some("Balancer crashed during startup".to_owned()),
    };
    let mut simulator = simulator(view_home(&data));

    match simulator.find("Balancer crashed during startup") {
        Ok(_) => Ok(()),
        Err(error) => bail!("expected to find the error text in the rendered tree: {error}"),
    }
}
