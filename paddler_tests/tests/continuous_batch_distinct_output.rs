#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn two_concurrent_prompts_produce_distinct_outputs() -> Result<()> {
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

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: vec![AgentConfig {
                name: "test-agent".to_owned(),
                slot_count: 2,
            }],
            desired_state: Some(desired_state),
            wait_for_slots_ready: true,
        },
    )
    .await?;

    let params_a = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 20,
        raw_prompt: "Count from one to ten in English: one, two,".to_owned(),
    };
    let params_b = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 20,
        raw_prompt: "The capital of France is".to_owned(),
    };
    let (collected_a, collected_b) = tokio::join!(
        cluster
            .inference_client
            .http()
            .continue_from_raw_prompt_collected(&params_a),
        cluster
            .inference_client
            .http()
            .continue_from_raw_prompt_collected(&params_b),
    );

    let collected_a = collected_a?;
    let collected_b = collected_b?;

    assert!(
        !collected_a.text.is_empty(),
        "first concurrent prompt should produce tokens"
    );
    assert!(
        !collected_b.text.is_empty(),
        "second concurrent prompt should produce tokens"
    );
    assert_ne!(
        collected_a.text, collected_b.text,
        "two different prompts should produce different outputs"
    );

    cluster.shutdown().await?;

    Ok(())
}
