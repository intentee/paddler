#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_applied_to_embedding_model() -> Result<()> {
    let ModelCard { reference, .. } = nomic_embed_text_v1_5();

    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: Some(chat_template.clone()),
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: true,
        }),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let retrieved = cluster
        .paddler_client
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read chat template override")?;

    assert_eq!(retrieved, Some(chat_template));

    cluster.shutdown().await?;

    Ok(())
}
