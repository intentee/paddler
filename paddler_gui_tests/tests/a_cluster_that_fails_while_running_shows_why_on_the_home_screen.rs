use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_gui::home_message::HomeMessage;
use paddler_gui::message::Message;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_gui_tests::loopback_balancer_addresses::LOOPBACK_BALANCER_ADDRESSES;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_service_thread::service_thread_error::ServiceThreadError;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_that_fails_while_running_shows_why_on_the_home_screen() {
    let mut app = AppDriver::open().await.expect("the app must open");

    for message in [
        Message::Home(HomeMessage::StartBalancer),
        Message::BalancerStarted {
            addresses: LOOPBACK_BALANCER_ADDRESSES,
            cancellation_token: CancellationToken::new(),
            snapshot: Box::new(RunningBalancerSnapshot {
                agent_snapshots: Vec::new(),
                balancer_applicable_state: BalancerApplicableState::from(
                    BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
                ),
                balancer_desired_state: BalancerDesiredState::unconfigured(
                    InferenceMode::TextGeneration,
                ),
            }),
        },
    ] {
        let _transition_task = app.update(message);
    }

    app.find("Cluster is running")
        .expect("a started cluster must show that it is running");

    let _failure_task = app.update(Message::BalancerFailed(Arc::new(
        BalancerRunnerError::ServiceThread(ServiceThreadError::ServiceThreadPanicked),
    )));

    app.find("A Paddler service thread panicked")
        .expect("a cluster that fails while running must show why on the home screen");
}
