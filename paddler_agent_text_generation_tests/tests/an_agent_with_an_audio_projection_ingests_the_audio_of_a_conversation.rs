#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::audio_transcription_conversation::audio_transcription_conversation;
use paddler_balancer_desired_state_fixture::ultravox_desired_state::ultravox_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::constants::QUICK_BROWN_FOX_WAV;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_with_an_audio_projection_ingests_the_audio_of_a_conversation() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(ultravox_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("an agent with an audio projection must start");

    let summary = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &audio_transcription_conversation(
                QUICK_BROWN_FOX_WAV
                    .input_audio()
                    .expect("the test audio must load"),
            ),
        )
        .await
        .expect("the inference request must be accepted")
        .summary()
        .expect("the conversation carrying audio must be generated to its end");

    assert!(summary.usage.input_audio > 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
