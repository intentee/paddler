#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::image_description_conversation::image_description_conversation;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::smolvlm2_desired_state::smolvlm2_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

const CHAT_TEMPLATE_RENDERING_ONLY_ROLES: &str =
    "{%- for message in messages -%}{{ message['role'] }}\n{% endfor -%}";

#[tokio::test(flavor = "multi_thread")]
async fn a_multimodal_agent_whose_chat_template_drops_the_image_rejects_the_prompt() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(
            desired_state_with_chat_template_override(
                smolvlm2_desired_state(),
                ChatTemplate {
                    content: CHAT_TEMPLATE_RENDERING_ONLY_ROLES.to_owned(),
                },
            ),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start serving");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: image_description_conversation(
                    LLAMAS_JPEG.data_uri().expect("the test image must load"),
                ),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::MultimodalTokenizationFailed(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
