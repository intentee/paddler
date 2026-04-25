#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::state_database_file::StateDatabaseFile;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_model_switch_in_storage() -> Result<()> {
    let database = StateDatabaseFile::new()?;

    let ModelCard { reference, .. } = qwen3_0_6b();

    let initial_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: Some(initial_state.clone()),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let observed_initial = cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read initial desired state")?;

    assert_eq!(observed_initial.model, initial_state.model);

    let switched_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("/tmp/alternative-model.gguf".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    cluster
        .paddler_client
        .management()
        .put_balancer_desired_state(&switched_state)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to switch desired model")?;

    let observed_switched = cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read switched desired state")?;

    assert_eq!(observed_switched.model, switched_state.model);

    cluster.shutdown().await?;

    Ok(())
}
