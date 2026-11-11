#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_decision_cluster_that_answered_a_decision::start_decision_cluster_that_answered_a_decision;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agents_keep_generating_text_once_the_pointer_head_is_removed() {
    let mut cluster = start_decision_cluster_that_answered_a_decision()
        .await
        .expect("a decision cluster must start and answer a decision");

    cluster
        .remove_pointer_head()
        .await
        .expect("the agent must stop serving decisions once the pointer head is removed");

    let generated = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: MAX_TOKENS,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the cluster without a pointer head must generate tokens");

    assert_ne!(generated.text, "");
    generated
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
