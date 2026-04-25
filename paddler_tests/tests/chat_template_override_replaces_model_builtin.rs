#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_replaces_model_builtin() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        wait_for_slots_ready: true,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: Some(chat_template.clone()),
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
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

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("The capital of France is".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 10,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let received_tokens = collected
        .token_results
        .iter()
        .any(|result| matches!(result, GeneratedTokenResult::Token(_)));

    assert!(
        received_tokens,
        "override template should render the prompt and produce tokens"
    );

    cluster.shutdown().await?;

    Ok(())
}
