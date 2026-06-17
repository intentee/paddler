#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::state_database_file::StateDatabaseFile;

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

    let cluster = Cluster::start(
        &InProcessClusterBackend::default().with_service_config(BalancerServiceConfig {
            state_database_url: database.url.clone(),
            ..Default::default()
        }),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(initial_state.clone()),
        },
    )
    .await?;

    let observed_initial = cluster
        .management_client
        .desired_state()
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
        .management_client
        .set_desired_state(&switched_state)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to switch desired model")?;

    let observed_switched = cluster
        .management_client
        .desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read switched desired state")?;

    assert_eq!(observed_switched.model, switched_state.model);

    cluster.shutdown().await?;

    Ok(())
}
