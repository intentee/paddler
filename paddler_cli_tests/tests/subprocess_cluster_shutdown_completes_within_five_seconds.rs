
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;
use tokio::time::timeout;

const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_shutdown_completes_within_five_seconds() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: AgentConfig::uniform(1, 4),
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    timeout(SHUTDOWN_BUDGET, cluster.shutdown())
        .await
        .with_context(|| {
            format!(
                "subprocess cluster shutdown did not complete within {SHUTDOWN_BUDGET:?}; \
                 SIGTERM was sent but at least one child paddler process did not exit in time, \
                 or balancer service drain did not return promptly"
            )
        })??;

    Ok(())
}
