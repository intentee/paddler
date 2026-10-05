#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::unending_grammar::unending_grammar;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_drains_in_flight_inference_before_swap() {
    let mut cluster = start_cluster_with_qwen3_and_context_size(
        AgentConfig::uniform(1, 1),
        CONTEXT_SIZE_THAT_ENDS_THE_IN_FLIGHT_GENERATION,
    )
    .await
    .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();
    let swapped_template = ChatTemplate {
        content: "PREFIX:{{ messages[0].content }}".to_owned(),
    };
    let in_flight_stream = cluster
        .continue_from_conversation_history_stream(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "The capital of France is".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(unending_grammar()),
                max_tokens: NonZeroU32::MAX,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");
    let initial_desired_state = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the balancer must report its desired state");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                chat_template_override: Some(swapped_template.clone()),
                use_chat_template_override: true,
                ..initial_desired_state
            },
        )
        .await
        .expect("the balancer must accept the desired state");
    cluster
        .wait_for_chat_template_override_in_use(&agent_id)
        .await
        .expect("the agent must use the chat template override");

    assert_eq!(
        collect_generated_tokens(in_flight_stream)
            .await
            .expect("the generated tokens must be collected")
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );
    assert_eq!(
        cluster
            .client_management
            .get_chat_template_override(CancellationToken::new(), &agent_id)
            .await
            .expect("the agent must report its chat template override"),
        Some(swapped_template)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
