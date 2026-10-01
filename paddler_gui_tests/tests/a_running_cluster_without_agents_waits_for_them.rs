use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_gui::running_balancer_data::RunningBalancerData;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_gui_tests::loopback_balancer_addresses::LOOPBACK_BALANCER_ADDRESSES;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[test]
fn a_running_cluster_without_agents_waits_for_them() {
    let running_balancer_data = RunningBalancerData {
        addresses: LOOPBACK_BALANCER_ADDRESSES,
        cancellation_token: CancellationToken::new(),
        snapshot: Box::new(RunningBalancerSnapshot {
            agent_snapshots: Vec::new(),
            balancer_applicable_state: BalancerApplicableState::from(
                BalancerDesiredState::default(),
            ),
            balancer_desired_state: BalancerDesiredState::default(),
        }),
        stopping: false,
    };
    let mut running_cluster = simulator(running_balancer_data.view());

    running_cluster
        .find("Waiting for agents to connect...")
        .expect("a running cluster without agents must wait for them");
}
