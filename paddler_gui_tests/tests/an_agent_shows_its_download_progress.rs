use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_gui::agent_running_data::AgentRunningData;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;
use paddler_messaging::model_download_status::ModelDownloadStatus;

#[test]
fn an_agent_shows_its_download_progress() {
    let agent_running_data = AgentRunningData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        balancer_connection: BalancerConnection::Connected,
        cancellation_token: CancellationToken::new(),
        name: None,
        slots_processing: 0,
        status: AgentStatus {
            download_status: ModelDownloadStatus::Downloading {
                downloaded_bytes: 250,
                model_path: "owner/repo/main/model.gguf".to_owned(),
                total_bytes: 1000,
            },
            ..AgentStatus::default()
        },
    };
    let mut running_agent = simulator(agent_running_data.view());

    running_agent
        .find("Status: Downloading (25%)")
        .expect("a downloading agent must show its download progress");
}
