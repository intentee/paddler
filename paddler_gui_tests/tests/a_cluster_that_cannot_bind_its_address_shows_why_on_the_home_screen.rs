use std::net::TcpListener;

use futures::StreamExt as _;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_gui::balancer_runner_messages::balancer_runner_messages;
use paddler_gui::home_message::HomeMessage;
use paddler_gui::message::Message;
use paddler_gui::start_balancer_form_action::StartBalancerFormAction;
use paddler_gui::start_balancer_form_data::StartBalancerFormData;
use paddler_gui::start_balancer_form_message::StartBalancerFormMessage;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_that_cannot_bind_its_address_shows_why_on_the_home_screen() {
    let occupying_listener =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("the occupying listener must bind");
    let occupied_addr = occupying_listener
        .local_addr()
        .expect("the occupying listener must report its address");
    let mut start_form = StartBalancerFormData::suggesting_addresses_on(None);

    for message in [
        StartBalancerFormMessage::SetBalancerAddress(EPHEMERAL_LOOPBACK_ADDR.to_string()),
        StartBalancerFormMessage::SetInferenceAddress(EPHEMERAL_LOOPBACK_ADDR.to_string()),
        StartBalancerFormMessage::ToggleAddModelLater(true),
    ] {
        let _form_action = start_form.update(message);
    }

    let StartBalancerFormAction::StartBalancer(mut bootstrap_config) =
        start_form.update(StartBalancerFormMessage::Confirm)
    else {
        panic!("a valid start form must start the cluster");
    };

    bootstrap_config.management_service_configuration.addr =
        ResolvedSocketAddr::from(occupied_addr);

    let cluster_messages = balancer_runner_messages(BalancerRunnerParams {
        bootstrap_config: *bootstrap_config,
        cancellation_token: CancellationToken::new(),
        shutdown_options: ServiceShutdownOptions::default(),
    })
    .collect::<Vec<Message>>()
    .await;
    let mut app = AppDriver::open().await.expect("the app must open");
    let _form_task = app.update(Message::Home(HomeMessage::StartBalancer));

    for message in cluster_messages {
        let _failure_task = app.update(message);
    }

    app.find(&format!(
        "Unable to bind the management service to {occupied_addr}"
    ))
    .expect("a cluster that cannot bind its address must show why on the home screen");
}
