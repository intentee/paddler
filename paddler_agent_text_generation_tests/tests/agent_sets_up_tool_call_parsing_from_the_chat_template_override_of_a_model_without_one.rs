#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::get_weather_tool::get_weather_tool;
use paddler_agent_text_generation_tests::qwen3_with_a_patched_chat_template::Qwen3WithAPatchedChatTemplate;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::generation_done_discriminant::generation_done_discriminant;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

const TEMPLATE_CALLING_ONE_TOOL_PER_TURN: &str = "{%- if tools %}{{ tools[0].function.name }}\n{% endif -%}{%- for message in messages -%}{{ message.role }}: {{ message.content }}{%- if message.tool_calls %} {{ message.tool_calls[0].function.name }}{% endif %}\n{% endfor -%}{%- if add_generation_prompt %}assistant: {% endif -%}";

#[tokio::test(flavor = "multi_thread")]
async fn agent_sets_up_tool_call_parsing_from_the_chat_template_override_of_a_model_without_one() {
    let qwen3_without_a_chat_template = Qwen3WithAPatchedChatTemplate::without_a_chat_template()
        .expect("the model without a chat template must be written");
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(
            desired_state_with_chat_template_override(
                qwen3_without_a_chat_template
                    .desired_state()
                    .expect("the model without a chat template must be at an absolute path"),
                ChatTemplate {
                    content: TEMPLATE_CALLING_ONE_TOOL_PER_TURN.to_owned(),
                },
            ),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text("Hello".to_owned()),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: ModelDefaultOr::Explicit(false),
                grammar: None,
                max_tokens: NonZeroU32::new(4).unwrap(),
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
                parse_tool_calls: true,
                tools: vec![get_weather_tool()],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected
            .token_results
            .last()
            .map(|token_result_with_producer| discriminant(
                &token_result_with_producer.token_result
            )),
        Some(generation_done_discriminant())
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
