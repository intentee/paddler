#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::ministral_3_cluster_params::Ministral3ClusterParams;
use paddler_tests::start_cluster_with_ministral_3::start_cluster_with_ministral_3;

const MINISTRAL_3_BOS_TOKEN_TEXT: &str = "<s>";
const RAW_PROMPT_WITHOUT_BOS: &str = "[INST]hi[/INST]";

#[tokio::test(flavor = "multi_thread")]
async fn agent_tokenizes_a_raw_prompt_opening_with_the_bos_token_with_a_single_bos_token() {
    let cluster = start_cluster_with_ministral_3(Ministral3ClusterParams::default())
        .await
        .expect("the cluster must start");
    let prompt_tokens_of = async |raw_prompt: String| {
        cluster
            .continue_from_raw_prompt(
                CancellationToken::new(),
                &ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::new(1).unwrap(),
                    raw_prompt,
                },
            )
            .await
            .expect("the inference request must be accepted")
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .prompt_tokens
    };

    let prompt_tokens_with_rendered_bos = prompt_tokens_of(format!(
        "{MINISTRAL_3_BOS_TOKEN_TEXT}{RAW_PROMPT_WITHOUT_BOS}"
    ))
    .await;
    let prompt_tokens_without_rendered_bos =
        prompt_tokens_of(RAW_PROMPT_WITHOUT_BOS.to_owned()).await;

    assert_eq!(
        prompt_tokens_with_rendered_bos,
        prompt_tokens_without_rendered_bos
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
