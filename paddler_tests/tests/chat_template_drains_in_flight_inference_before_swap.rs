#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_client::collect_generated_tokens::collect_generated_tokens;
use paddler_client::token_result_with_producer::TokenResultWithProducer;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_drains_in_flight_inference_before_swap() -> Result<()> {
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

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
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
        },
    )
    .await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have one registered agent")?;

    let in_flight_stream = cluster
        .inference_client
        .http()
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
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
        })
        .await?;

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
        .management_client
        .set_desired_state(&swap_state)
        .await
        .map_err(anyhow::Error::new)?;

    let collected = collect_generated_tokens(in_flight_stream).await?;

    assert!(
        collected
            .token_results
            .iter()
            .any(|result| result.token_result.is_token()),
        "in-flight request must continue producing tokens during template swap"
    );

    assert!(
        !collected.token_results.iter().any(|result| matches!(
            result.token_result,
            GeneratedTokenResult::ChatTemplateError(_)
        )),
        "in-flight request must not see ChatTemplateError during swap"
    );

    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    let retrieved = cluster
        .management_client
        .chat_template_override(&agent_id)
        .await
        .map_err(anyhow::Error::new)?;

    assert_eq!(retrieved, Some(template_b));

    cluster.shutdown().await?;

    Ok(())
}
