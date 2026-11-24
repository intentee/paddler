#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_answers_model_metadata_of_its_loaded_model() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have at least one registered agent")
        .clone();

    let metadata = cluster
        .client_management
        .get_model_metadata(CancellationToken::new(), &agent_id)
        .await
        .expect("get_model_metadata should succeed")
        .expect("model metadata should be present for a loaded agent");

    assert!(
        !metadata.metadata.is_empty(),
        "model metadata map must not be empty"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
