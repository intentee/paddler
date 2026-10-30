#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::subprocess_cluster_with_a_serving_and_a_stuck_agent::SERVING_AGENT_NAME;
use crate::subprocess_cluster_with_a_serving_and_a_stuck_agent::subprocess_cluster_with_a_serving_and_a_stuck_agent;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_dispatches_requests_only_to_agents_that_serve() {
    let cluster = subprocess_cluster_with_a_serving_and_a_stuck_agent().await;

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
        .expect("the generation must be answered");

    assert!(
        collected
            .token_results
            .iter()
            .all(|token_result| token_result.generated_by.as_deref() == Some(SERVING_AGENT_NAME))
    );

    collected
        .summary()
        .expect("the serving agent must finish the generation");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
