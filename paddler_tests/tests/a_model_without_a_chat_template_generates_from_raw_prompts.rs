#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

use crate::qwen3_without_a_chat_template::Qwen3WithoutAChatTemplate;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_without_a_chat_template_generates_from_raw_prompts() {
    let qwen3_without_a_chat_template = Qwen3WithoutAChatTemplate::write();
    let cluster = start_single_agent_cluster_with_desired_state(
        qwen3_without_a_chat_template.desired_state(),
    )
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the raw prompt must be accepted");

    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
