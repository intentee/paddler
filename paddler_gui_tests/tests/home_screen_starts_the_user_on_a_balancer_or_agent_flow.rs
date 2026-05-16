use anyhow::Result;
use anyhow::bail;
use iced_test::simulator;
use paddler_gui::home_data::HomeData;
use paddler_gui::home_handler::Message;
use paddler_gui::ui::view_home::view_home;

#[test]
fn clicking_start_a_cluster_takes_the_user_to_the_start_balancer_form() -> Result<()> {
    let data = HomeData { error: None };
    let mut simulator = simulator(view_home(&data));

    simulator.click("Start a cluster")?;

    match simulator.into_messages().next() {
        Some(Message::StartBalancer) => Ok(()),
        other => bail!("expected StartBalancer message, got {other:?}"),
    }
}

#[test]
fn clicking_join_a_cluster_takes_the_user_to_the_join_balancer_form() -> Result<()> {
    let data = HomeData { error: None };
    let mut simulator = simulator(view_home(&data));

    simulator.click("Join a cluster")?;

    match simulator.into_messages().next() {
        Some(Message::JoinBalancer) => Ok(()),
        other => bail!("expected JoinBalancer message, got {other:?}"),
    }
}
