use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_gui::agent_running_data::AgentRunningData;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;

#[test]
fn a_joining_agent_says_it_is_connecting() {
    let agent_running_data = AgentRunningData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        balancer_connection: BalancerConnection::Connecting,
        cancellation_token: CancellationToken::new(),
        name: Some("gpu-box".to_owned()),
        slots_processing: 0,
        status: AgentStatus::default(),
    };
    let mut running_agent = simulator(agent_running_data.view());

    running_agent
        .find("Connecting to the cluster...")
        .expect("a joining agent must say that it is connecting");
}
