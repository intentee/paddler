#![cfg(feature = "tests_that_use_llms")]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::audio_transcription_conversation::audio_transcription_conversation;
use paddler_balancer_desired_state_fixture::ultravox_desired_state::ultravox_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::audio_format::AudioFormat;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::input_audio::InputAudio;
use paddler_messaging::media_kind::MediaKind;
use paddler_messaging::undecodable_media::UndecodableMedia;

#[tokio::test(flavor = "multi_thread")]
async fn audio_llama_cpp_cannot_read_is_refused_as_undecodable_media() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(ultravox_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("an agent with an audio projection must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &audio_transcription_conversation(InputAudio {
                data: BASE64_STANDARD.encode("these bytes are neither audio nor an image"),
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
