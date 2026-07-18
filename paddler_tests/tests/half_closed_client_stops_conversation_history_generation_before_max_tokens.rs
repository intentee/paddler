#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

const MAX_TOKENS_TOO_MANY_TO_FINISH_INSIDE_THE_OBSERVATION_WINDOW: i32 = 4096;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_client_stops_conversation_history_generation_before_max_tokens() -> Result<()>
{
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        wait_for_slots_ready: true,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let params: ContinueFromConversationHistoryParams<RawParametersSchema> =
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "Write a very long story about a dragon".to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: MAX_TOKENS_TOO_MANY_TO_FINISH_INSIDE_THE_OBSERVATION_WINDOW,
            parse_tool_calls: false,
            tools: Vec::new(),
        };

    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.inference,
        "/api/v1/continue_from_conversation_history",
        &params,
    )
    .await?;

    cluster
        .wait_for_slots_processing(&agent_id, 1, ObservationWindow::model_load())
        .await
        .context("the agent must start generating before the client goes away")?;

    client.half_close().await?;

    cluster
        .wait_for_slots_processing(&agent_id, 0, ObservationWindow::release())
        .await
        .context(
            "a half-closed client must stop generation; with inference_item_timeout set to an \
             hour the slot can only be released once the agent confirms that it stopped",
        )?;

    drop(client);

    cluster.shutdown().await?;

    Ok(())
}
