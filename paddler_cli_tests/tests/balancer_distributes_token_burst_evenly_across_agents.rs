#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use futures_util::StreamExt as _;
use futures_util::future;
use futures_util::stream::iter;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::unending_generation::unending_generation;

const AGENT_COUNT: usize = 4;
const SLOTS_PER_AGENT: u16 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_token_burst_evenly_across_agents() {
    let agent_configs = AgentConfig::uniform(AGENT_COUNT, SLOTS_PER_AGENT);
    let cluster = start_subprocess_cluster_with_qwen3(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        agent_configs.clone(),
    )
    .await
    .expect("the cluster must start");
    let generation_cancellation = CancellationToken::new();
    let mut streams = future::try_join_all((0..AGENT_COUNT).map(|_generation_index| {
        cluster.continue_from_raw_prompt_stream(
            generation_cancellation.clone(),
            &unending_generation(),
        )
    }))
    .await
    .expect("every concurrent request must succeed");
    let mut first_streamed_items = Vec::new();

    for stream in &mut streams {
        first_streamed_items.push(
            stream
                .next()
                .await
                .expect("every held generation must stream a first message"),
        );
    }

    generation_cancellation.cancel();
    drop(streams);

    let producers =
        future::try_join_all(first_streamed_items.into_iter().map(|first_streamed_item| {
            collect_generated_tokens(Box::pin(iter([first_streamed_item])))
        }))
        .await
        .expect("every held generation must begin with a generated token")
        .into_iter()
        .flat_map(|collected| collected.token_results)
        .map(|token_result_with_producer| token_result_with_producer.generated_by)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        producers,
        agent_configs
            .into_iter()
            .map(|agent_config| Some(agent_config.name))
            .collect::<BTreeSet<_>>()
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
