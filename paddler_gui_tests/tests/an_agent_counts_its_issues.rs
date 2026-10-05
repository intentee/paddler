use std::collections::BTreeSet;

use iced_test::simulator;
use tokio_util::sync::CancellationToken;

use paddler_gui::agent_running_data::AgentRunningData;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;

#[test]
fn an_agent_counts_its_issues() {
    let missing_model = ModelPath {
        model_path: "/models/qwen3.gguf".to_owned(),
    };
    let agent_running_data = AgentRunningData {
        balancer_address: "127.0.0.1:8060".to_owned(),
        balancer_connection: BalancerConnection::Connected,
        cancellation_token: CancellationToken::new(),
        name: Some("gpu-box".to_owned()),
        slots_processing: 0,
        status: AgentStatus {
            issues: BTreeSet::from([
                AgentIssue::ModelFileDoesNotExist(missing_model.clone()),
                AgentIssue::UnableToFindChatTemplate(missing_model),
            ]),
            ..AgentStatus::default()
        },
    };
    let mut running_agent = simulator(agent_running_data.view());

    running_agent
        .find("2 issues")
        .expect("an agent with issues must count them");
}
