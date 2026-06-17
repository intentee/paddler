#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_client::collect_generated_tokens::collect_generated_tokens;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn management_two_agents_stream_subscribers_receive_slot_usage_changes() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::default()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let mut cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: vec![AgentConfig {
                name: "test-agent".to_owned(),
                slot_count: 1,
            }],
            desired_state: DesiredStateInit::set(desired_state),
            wait_for_slots_ready: true,
        },
    )
    .await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have registered one agent")?;

    let token_stream = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 8,
            raw_prompt: "Count to three".to_owned(),
        })
        .await?;

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .context("agents_stream must emit a snapshot showing slot usage")?;

    collect_generated_tokens(token_stream).await?;

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .context("agents_stream must emit a snapshot showing the slot was released")?;

    cluster.shutdown().await?;

    Ok(())
}
