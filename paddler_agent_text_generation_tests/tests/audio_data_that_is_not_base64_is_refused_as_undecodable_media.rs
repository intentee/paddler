#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::audio_transcription_conversation::audio_transcription_conversation;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;
use paddler_messaging::audio_format::AudioFormat;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::input_audio::InputAudio;
use paddler_messaging::media_kind::MediaKind;
use paddler_messaging::undecodable_media::UndecodableMedia;

#[tokio::test(flavor = "multi_thread")]
async fn audio_data_that_is_not_base64_is_refused_as_undecodable_media() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &audio_transcription_conversation(InputAudio {
                data: "!!!not-valid-base64!!!".to_owned(),
                format: AudioFormat::Wav,
            }),
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::MediaDecodingFailed(
            UndecodableMedia {
                media: MediaKind::Audio,
                ..
            }
        )]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
