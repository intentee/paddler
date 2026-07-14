#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_chat_template_override_across_restart() -> Result<()> {
    let database = StateDatabaseFile::new()?;

    let ModelCard { reference, .. } = qwen3_0_6b();

    let template_content = "{{ messages | tojson }}".to_owned();

    let desired_state = BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: template_content.clone(),
        }),
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    let first_cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: Some(desired_state.clone()),
        ..ClusterParams::default()
    })
    .await?;

    first_cluster.shutdown().await?;

    let second_cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: None,
        ..ClusterParams::default()
    })
    .await?;

    let restored_state = second_cluster
        .client_management
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read restored desired state")?;

    assert!(restored_state.use_chat_template_override);
    assert_eq!(
        restored_state
            .chat_template_override
            .as_ref()
            .map(|template| template.content.as_str()),
        Some(template_content.as_str())
    );

    second_cluster.shutdown().await?;

    Ok(())
}
