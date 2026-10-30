use std::io::Write as _;

use tempfile::NamedTempFile;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_model_cannot_start_reports_that_it_is_stuck() {
    let mut unloadable_model =
        NamedTempFile::new().expect("a temporary model file must be creatable");

    unloadable_model
        .write_all(b"this is not a valid gguf model file")
        .expect("the unloadable model contents must be writable");

    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: agent_desired_model_at(unloadable_model.path())
            .expect("the temporary model path must be absolute"),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have one registered agent")
        .clone();

    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot.agents.iter().any(|registered_agent| {
                registered_agent.id == agent_id
                    && registered_agent.status.state_application_status
                        == AgentStateApplicationStatus::Stuck
            })
        })
        .await
        .expect("the agent must report that its pipeline is stuck after retrying");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
