#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::chat_template_rendering_every_message_after::chat_template_rendering_every_message_after;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::image_description_conversation::image_description_conversation;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_multimodal_agent_serves_a_template_that_requires_a_system_message() {
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(
            desired_state_with_chat_template_override(
                smolvlm2_256m().into_desired_state_with_multimodal_projection(smolvlm2_256m_mmproj()),
                chat_template_rendering_every_message_after(
                    "{% if messages[0]['role'] != 'system' %}{{ raise_exception('the first message must be the system message') }}{% endif %}",
                ),
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
                    load_test_image_data_uri().expect("the test image must load"),
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

    collected
        .summary()
        .expect("the conversation opening with a system message must be generated to its end");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
