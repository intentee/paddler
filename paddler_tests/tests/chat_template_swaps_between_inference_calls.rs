#![cfg(feature = "tests_that_use_llms")]

use std::future::Future;

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

fn run_inference_after_template_swap(
    cluster: &Cluster,
) -> impl Future<Output = Result<bool>> + Send + use<> {
    let generation = cluster.continue_from_conversation_history(
        CancellationToken::new(),
        &ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("The capital of France is".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 10,
            parse_tool_calls: false,
            tools: vec![],
        },
    );

    async move {
        let collected = generation.await?;

        Ok(collected
            .token_results
            .iter()
            .any(|result| result.token_result.is_token()))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_swaps_between_inference_calls() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let template_a = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };
    let template_b = ChatTemplate {
        content: "PREFIX:{{ messages[0].content }}".to_owned(),
    };

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: true,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: Some(template_a.clone()),
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference.clone()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: true,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    assert!(
        run_inference_after_template_swap(&cluster).await?,
        "first inference with template_a must produce tokens"
    );

    let swap_state = BalancerDesiredState {
        chat_template_override: Some(template_b.clone()),
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::default()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &swap_state)
        .await
        .map_err(anyhow::Error::new)?;

    assert!(
        run_inference_after_template_swap(&cluster).await?,
        "inference after swap must produce tokens with template_b"
    );

    let retrieved = cluster
        .client_management
        .get_chat_template_override(CancellationToken::new(), &agent_id)
        .await
        .map_err(anyhow::Error::new)?;

    assert_eq!(retrieved, Some(template_b));

    cluster.shutdown().await?;

    Ok(())
}
