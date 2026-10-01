use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui_tests::loopback_balancer_addresses::LOOPBACK_BALANCER_ADDRESSES;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[test]
fn a_running_cluster_lists_its_agents() {
    let running_balancer_data = RunningBalancerData {
        addresses: LOOPBACK_BALANCER_ADDRESSES,
        cancellation_token: CancellationToken::new(),
        snapshot: Box::new(RunningBalancerSnapshot {
            agent_snapshots: vec![AgentControllerSnapshot {
                id: "agent_id".to_owned(),
                name: Some("gpu-box".to_owned()),
                slots_processing: 1,
                status: AgentStatus {
                    desired_slots_total: 4,
                    model_path: Some("/models/qwen3.gguf".to_owned()),
                    slots_total: 4,
                    state_application_status: AgentStateApplicationStatus::Applied,
                    ..AgentStatus::default()
                },
            }],
            balancer_applicable_state: BalancerApplicableState::from(
                BalancerDesiredState::default(),
            ),
            balancer_desired_state: BalancerDesiredState::default(),
        }),
        stopping: false,
    };
    let mut running_cluster = simulator(running_balancer_data.view());

    running_cluster
        .find("gpu-box")
        .expect("the running cluster must name its agent");
    running_cluster
        .find("qwen3.gguf")
        .expect("the running cluster must show the model its agent loaded");
    running_cluster
        .find("Status: OK")
        .expect("the running cluster must show the status of its agent");
    running_cluster
        .find("Slots: 1/4/4")
        .expect("the running cluster must show the slots of its agent");
}
