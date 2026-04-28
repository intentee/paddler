#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_huggingface_mmproj_in_desired_state() -> Result<()> {
    let ModelCard {
        reference: primary_reference,
        ..
    } = smolvlm2_256m();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = smolvlm2_256m_mmproj();

    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference.clone()),
            use_chat_template_override: false,
        }),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let retrieved = cluster
        .paddler_client
        .management()
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read balancer desired state")?;

    assert_eq!(
        retrieved.multimodal_projection,
        AgentDesiredModel::HuggingFace(mmproj_reference)
    );

    cluster.shutdown().await?;

    Ok(())
}
