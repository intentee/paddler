#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_local_mmproj_path_in_desired_state() -> Result<()> {
    let ModelCard { reference, .. } = smolvlm2_256m();

    let local_mmproj_path = "/tmp/test-mmproj.gguf".to_owned();

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::LocalToAgent(local_mmproj_path.clone()),
                use_chat_template_override: false,
            }),
        },
    )
    .await?;

    let retrieved = cluster
        .management_client
        .desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read balancer desired state")?;

    assert_eq!(
        retrieved.multimodal_projection,
        AgentDesiredModel::LocalToAgent(local_mmproj_path)
    );

    cluster.shutdown().await?;

    Ok(())
}
