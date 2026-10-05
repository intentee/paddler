use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_gui::balancer_launch::BalancerLaunch;
use paddler_gui::start_balancer_form_data::StartBalancerFormData;

#[test]
fn start_form_cannot_be_submitted_while_the_cluster_starts() {
    let start_form_data = StartBalancerFormData {
        launch: BalancerLaunch::Starting(CancellationToken::new()),
        ..StartBalancerFormData::suggesting_addresses_on(None)
    };
    let mut start_form = simulator(start_form_data.view());

    start_form
        .click("Starting...")
        .expect("the start form must show that the cluster is starting");

    assert!(start_form.into_messages().next().is_none());
}
