#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use anyhow::Result;
use anyhow::anyhow;
use futures_util::future;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_token_burst_evenly_across_agents() -> Result<()> {
    const AGENT_COUNT: usize = 4;
    const SLOTS_PER_AGENT: i32 = 1;

    let cluster = start_subprocess_cluster_with_qwen3(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        AgentConfig::uniform(AGENT_COUNT, SLOTS_PER_AGENT),
    )
    .await?;

    let prompts: Vec<String> = (0..AGENT_COUNT)
        .map(|index| format!("Burst request number {index}: Count from one to five."))
        .collect();

    let collection_futures = prompts.iter().map(|prompt| {
        cluster.continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: prompt.clone(),
        })
    });

    let collected_streams = future::try_join_all(collection_futures).await?;

    let mut producer_per_stream: Vec<String> = Vec::with_capacity(AGENT_COUNT);

    for (stream_index, collected) in collected_streams.iter().enumerate() {
        let producers_for_stream: BTreeSet<&str> = collected
            .token_results
            .iter()
            .filter_map(|chunk| chunk.generated_by.as_deref())
            .collect();

        assert_eq!(
            producers_for_stream.len(),
            1,
            "stream {stream_index} must be served by exactly one agent, but saw producers: {producers_for_stream:?}",
        );

        let producer = producers_for_stream
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("stream {stream_index} produced no attributable tokens"))?
            .to_owned();

        producer_per_stream.push(producer);
    }

    let unique_producers: BTreeSet<&str> = producer_per_stream.iter().map(String::as_str).collect();

    assert_eq!(
        unique_producers.len(),
        AGENT_COUNT,
        "burst of {AGENT_COUNT} requests with {SLOTS_PER_AGENT} slot per agent must fan out across all agents, but stream-to-producer map was: {producer_per_stream:?}",
    );

    cluster.shutdown().await?;

    Ok(())
}
