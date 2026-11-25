#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::chat_template_rendering_every_message_after::chat_template_rendering_every_message_after;
use paddler_agent_text_generation_tests::image_description_conversation::image_description_conversation;
use paddler_agent_text_generation_tests::single_image_question::SingleImageQuestion;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::gemma_3_4b_it::gemma_3_4b_it;
use paddler_model_card_fixture::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;

#[tokio::test(flavor = "multi_thread")]
async fn a_multimodal_prompt_carries_one_bos_whether_or_not_its_template_renders_it() {
    let image_data_uri = LLAMAS_JPEG.data_uri().expect("the test image must load");
    let conversations = [
        image_description_conversation(image_data_uri.clone()),
        SingleImageQuestion {
            image_data_uri,
            max_tokens: NonZeroU32::MIN,
            question: "What do you see in this image?",
        }
        .into_conversation_params()
        .conversation_history,
    ];
    let prompt_tokens_rendered_by = async |chat_template: ChatTemplate| -> Vec<u64> {
        let cluster = Cluster::start_in_process(ClusterParams {
            agents: vec![AgentConfig::single(1)],
            desired_state: ClusterDesiredState::Apply(Box::new(
                desired_state_with_chat_template_override(
                    gemma_3_4b_it()
                        .into_desired_state_with_multimodal_projection(gemma_3_4b_it_mmproj()),
                    chat_template,
                ),
            )),
            wait_for_slots_ready: true,
            ..ClusterParams::default()
        })
        .await
        .expect("the multimodal cluster must start");
        let mut prompt_tokens = Vec::with_capacity(conversations.len());

        for conversation_history in &conversations {
            prompt_tokens.push(
                cluster
                    .continue_from_conversation_history(
                        CancellationToken::new(),
                        &ContinueFromConversationHistoryParams {
                            add_generation_prompt: true,
                            conversation_history: conversation_history.clone(),
                            enable_thinking: false,
                            grammar: None,
                            max_tokens: NonZeroU32::MIN,
                            parse_tool_calls: false,
                            tools: vec![],
                        },
                    )
                    .await
                    .expect("the conversation must be served")
                    .summary()
                    .expect("the conversation must end with a summary")
                    .usage
                    .prompt,
            );
        }

        cluster
            .shutdown()
            .await
            .expect("the multimodal cluster must shut down");

        prompt_tokens
    };

    assert_eq!(
        prompt_tokens_rendered_by(chat_template_rendering_every_message_after(
            "{% if messages[0]['role'] == 'system' %}{{ bos_token }}{% endif %}",
        ))
        .await,
        prompt_tokens_rendered_by(chat_template_rendering_every_message_after(
            "{{ bos_token }}"
        ))
        .await
    );
}
