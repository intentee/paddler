#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3_embedding::start_subprocess_cluster_with_qwen3_embedding;
use paddler_types::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_starts_four_agents_within_sequential_spawn_budget() -> Result<()> {
    let agent_count: usize = 4;
    let single_agent_init_budget = Duration::from_secs(8);
    let cluster_overhead_budget = Duration::from_secs(8);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "agent_count is a fixed test constant that fits in u32"
    )]
    let cluster_startup_budget =
        single_agent_init_budget * (agent_count as u32) + cluster_overhead_budget;

    let cluster_startup_started_at = Instant::now();

    let cluster = start_subprocess_cluster_with_qwen3_embedding(Qwen3EmbeddingClusterParams {
        agents: AgentConfig::uniform(agent_count, 2),
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let cluster_startup_elapsed = cluster_startup_started_at.elapsed();

    assert_eq!(
        cluster.agent_ids.len(),
        agent_count,
        "expected {agent_count} agents to register; got {actual}",
        actual = cluster.agent_ids.len(),
    );

    cluster.shutdown().await?;

    assert!(
        cluster_startup_elapsed <= cluster_startup_budget,
        "cluster startup took {cluster_startup_elapsed:?}, expected within {cluster_startup_budget:?}. \
         Under concurrent agent spawn on Metal, kernel-compile contention can starve a single agent \
         for 60-120s. Sequential spawn isolates each agent's Metal init and keeps total startup \
         within {single_agent_init_budget:?} per agent plus {cluster_overhead_budget:?} of overhead."
    );

    Ok(())
}
