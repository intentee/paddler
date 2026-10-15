#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_test_cluster_harness::unending_grammar::unending_grammar;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_client_stops_conversation_history_generation_before_max_tokens() {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        wait_for_slots_ready: true,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
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
            grammar: Some(unending_grammar()),
            max_tokens: NonZeroU32::MAX,
            parse_tool_calls: false,
            tools: Vec::new(),
        };

    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.inference,
        ApiPath::CONTINUE_FROM_CONVERSATION_HISTORY,
        &params,
    )
    .await
    .expect("the half-closed request must be sent");

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("the agent must start generating before the client goes away");

    client
        .half_close()
        .await
        .expect("the request must be half-closed");

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect(
            "a half-closed client must stop generation; with inference_item_timeout set to an \
             hour the slot can only be released once the agent confirms that it stopped",
        );

    drop(client);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
