#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cli_tests::model_card::ModelCard;
use paddler_cli_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::state_database_file::StateDatabaseFile;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_desired_state_across_restart() -> Result<()> {
    let database = StateDatabaseFile::new()?;

    let ModelCard { reference, .. } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let first_cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: Some(desired_state.clone()),
        ..SubprocessClusterParams::default()
    })
    .await?;

    first_cluster.shutdown().await?;

    let second_cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: None,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let restored_state = second_cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read restored desired state")?;

    assert_eq!(restored_state.model, desired_state.model);

    second_cluster.shutdown().await?;

    Ok(())
}
