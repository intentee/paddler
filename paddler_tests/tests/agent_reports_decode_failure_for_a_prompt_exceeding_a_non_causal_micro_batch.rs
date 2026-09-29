#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use llama_cpp_bindings::context::params::LlamaContextParams;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::nomic_embed_desired_state_with_chat_template_override::nomic_embed_desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(1).unwrap();
const PROMPT_WORD: &str = "hello ";

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_decode_failure_for_a_prompt_exceeding_a_non_causal_micro_batch() -> Result<()>
{
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: Some(nomic_embed_desired_state_with_chat_template_override(
            ChatTemplate {
                content: "{{ messages[0].content }}".to_owned(),
            },
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;
    let micro_batch_tokens = usize::try_from(LlamaContextParams::default().n_ubatch())?;

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: MAX_TOKENS,
                raw_prompt: PROMPT_WORD.repeat(micro_batch_tokens + 1),
            },
        )
        .await?;

    let token_results = collected.into_token_results();

    assert!(matches!(
        token_results.as_slice(),
        [GeneratedTokenResult::DecodeFailed(_)]
    ));

    cluster.shutdown().await?;

    Ok(())
}
