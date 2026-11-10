#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::greeting_conversation::greeting_conversation;
use paddler_tests::start_cluster_with_gemma_3::start_cluster_with_gemma_3;

const RAW_PROMPT_WITHOUT_BOS: &str = "<start_of_turn>user\nhi<end_of_turn>\n<start_of_turn>model\n";

#[tokio::test(flavor = "multi_thread")]
async fn agent_tokenizes_gemma_3_chat_prompt_with_a_single_bos_token() {
    let cluster = start_cluster_with_gemma_3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let conversation = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &greeting_conversation(NonZeroU32::MIN),
        )
        .await
        .expect("the inference request must be accepted");
    let raw_prompt = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: RAW_PROMPT_WITHOUT_BOS.to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        conversation
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .prompt_tokens,
        raw_prompt
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .prompt_tokens
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
