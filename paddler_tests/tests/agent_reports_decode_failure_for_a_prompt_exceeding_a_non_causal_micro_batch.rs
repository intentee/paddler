#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::nomic_embed_desired_state_with_chat_template_override::nomic_embed_desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(1).unwrap();
const PROMPT_WORD: &str = "hello ";
const LLAMA_CPP_DEFAULT_MICRO_BATCH_TOKENS: usize = 512;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_decode_failure_for_a_prompt_exceeding_a_non_causal_micro_batch() {
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(
            nomic_embed_desired_state_with_chat_template_override(ChatTemplate {
                content: "{{ messages[0].content }}".to_owned(),
            })
            .expect("a text generation state must accept a chat template override"),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: MAX_TOKENS,
                raw_prompt: PROMPT_WORD.repeat(LLAMA_CPP_DEFAULT_MICRO_BATCH_TOKENS + 1),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let token_results = collected.into_token_results();

    assert!(matches!(
        token_results.as_slice(),
        [GeneratedTokenResult::DecodeFailed(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
