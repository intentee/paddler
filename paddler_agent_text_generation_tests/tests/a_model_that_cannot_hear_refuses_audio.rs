#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::audio_transcription_conversation::audio_transcription_conversation;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;
use paddler_file_fixture::constants::QUICK_BROWN_FOX_WAV;
use paddler_messaging::conversation_feature::ConversationFeature;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_that_cannot_hear_refuses_audio() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &audio_transcription_conversation(
                QUICK_BROWN_FOX_WAV
                    .input_audio()
                    .expect("the test audio must load"),
            ),
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::ConversationFeatureNotSupported(
            UnsupportedConversationFeature {
                feature: ConversationFeature::AudioInput,
                ..
            }
        )]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
