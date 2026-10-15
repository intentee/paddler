#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_keeps_serving_its_old_model_until_the_new_one_is_ready() {
    let new_model = smolvlm2_256m();
    let new_model_path = new_model
        .reference
        .cached_path(&Cache::from_env())
        .expect("the new model must be in the Hugging Face cache")
        .to_str()
        .expect("the new model path must be valid UTF-8")
        .to_owned();
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have one registered agent")
        .clone();
    let mut every_observed_agent_was_serving = true;

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &new_model.into_desired_state())
        .await
        .expect("the balancer must accept the desired state");
    cluster
        .agents_watcher
        .until_agent(&agent_id, |snapshot| {
            snapshot
                .agents
                .iter()
                .filter(|registered_agent| registered_agent.id == agent_id)
                .any(|registered_agent| {
                    every_observed_agent_was_serving &=
                        registered_agent.status.runtime != AgentRuntimeStatus::Idle;

                    registered_agent.status.model_path.as_deref() == Some(new_model_path.as_str())
                })
        })
        .await
        .expect("the agent must serve the new model");

    assert!(every_observed_agent_was_serving);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
