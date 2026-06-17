#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::state_database_file::StateDatabaseFile;

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

    let first_cluster = Cluster::start(
        &InProcessClusterBackend::default().with_service_config(BalancerServiceConfig {
            state_database_url: database.url.clone(),
            ..Default::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(desired_state.clone()),
        },
    )
    .await?;

    first_cluster.shutdown().await?;

    let second_cluster = Cluster::start(
        &InProcessClusterBackend::default().with_service_config(BalancerServiceConfig {
            state_database_url: database.url.clone(),
            ..Default::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::Inherit,
        },
    )
    .await?;

    let restored_state = second_cluster
        .management_client
        .desired_state()
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
