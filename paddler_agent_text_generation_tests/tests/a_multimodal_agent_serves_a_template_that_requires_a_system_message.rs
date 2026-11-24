#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::chat_template_rendering_every_message_after::chat_template_rendering_every_message_after;
use paddler_agent_text_generation_tests::image_description_conversation::image_description_conversation;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

#[tokio::test(flavor = "multi_thread")]
async fn a_multimodal_agent_serves_a_template_that_requires_a_system_message() {
    let cluster = Cluster::start_in_process(ClusterParams {
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

    collected
        .summary()
        .expect("the conversation opening with a system message must be generated to its end");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
