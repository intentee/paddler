use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui_tests::loopback_balancer_addresses::LOOPBACK_BALANCER_ADDRESSES;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

#[test]
fn a_running_cluster_shows_its_configured_and_applied_models() {
    let running_balancer_data = RunningBalancerData {
        addresses: LOOPBACK_BALANCER_ADDRESSES,
        cancellation_token: CancellationToken::new(),
        snapshot: Box::new(RunningBalancerSnapshot {
            agent_snapshots: Vec::new(),
            balancer_applicable_state: BalancerApplicableState::from(
                BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
            ),
            balancer_desired_state: BalancerDesiredState {
                model: AgentDesiredModel::LocalToAgent("/models/qwen3.gguf".to_owned()),
                ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
            },
        }),
        stopping: false,
    };
    let mut running_cluster = simulator(running_balancer_data.view());

    running_cluster
        .find("Configured model: Local: /models/qwen3.gguf")
        .expect("the running cluster must show the model it was configured with");
    running_cluster
        .find("Applied model: (not set)")
        .expect("the running cluster must show the model its agents apply");
}
