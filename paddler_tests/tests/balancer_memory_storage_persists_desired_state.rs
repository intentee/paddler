#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_memory_storage_persists_desired_state() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        state_database_url: "memory://".to_owned(),
        desired_state: Some(desired_state.clone()),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let observed = cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read desired state from memory backend")?;

    assert_eq!(observed.model, desired_state.model);

    cluster.shutdown().await?;

    Ok(())
}
