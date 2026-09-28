#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::future::join_all;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const CONCURRENT_REQUESTS: usize = 256;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_every_concurrent_request_on_a_single_slot_agent() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: Some(qwen3_desired_state()),
        max_buffered_requests: i32::try_from(CONCURRENT_REQUESTS)?,
        wait_for_slots_ready: true,
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let collected_requests = join_all((0..CONCURRENT_REQUESTS).map(|_| {
        cluster.continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 1,
                raw_prompt: "Hello".to_owned(),
            },
        )
    }))
    .await;

    for collected in collected_requests {
        assert!(matches!(
            collected?
                .token_results
                .last()
                .map(|token_result_with_producer| &token_result_with_producer.token_result),
            Some(GeneratedTokenResult::Done(_))
        ));
    }

    cluster.shutdown().await?;

    Ok(())
}
