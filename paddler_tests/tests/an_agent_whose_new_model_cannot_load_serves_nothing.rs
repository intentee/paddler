#![cfg(feature = "tests_that_use_llms")]

use std::io::Write as _;

use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_new_model_cannot_load_serves_nothing() {
    let mut unloadable_model =
        NamedTempFile::new().expect("a temporary model file must be creatable");

    unloadable_model
        .write_all(b"this is not a valid gguf model file")
        .expect("the unloadable model contents must be writable");

    let unloadable_model_path = unloadable_model
        .path()
        .to_str()
        .expect("the temporary model path must be valid UTF-8")
        .to_owned();
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have one registered agent")
        .clone();

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: agent_desired_model_at(unloadable_model.path())
                    .expect("the temporary model path must be absolute"),
                ..qwen3_desired_state()
            },
        )
        .await
        .expect("the balancer must accept the desired state");
    cluster
        .wait_for_first_agent_issue(|issue| {
            *issue
                == AgentIssue::ModelCannotBeLoaded(ModelPath {
                    model_path: unloadable_model_path.clone(),
                })
        })
        .await
        .expect("the agent must report that the new model cannot be loaded");
    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot.agents.iter().any(|registered_agent| {
                registered_agent.id == agent_id
                    && registered_agent.status.runtime == AgentRuntimeStatus::Idle
                    && registered_agent.status.model_path.is_none()
            })
        })
        .await
        .expect("the agent must serve nothing once the old model was released");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
