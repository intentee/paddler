use anyhow::Result;
use tokio::runtime::Builder;

use crate::agent_config::AgentConfig;
use crate::start_subprocess_cluster::start_subprocess_cluster;
use crate::subprocess_cluster_params::SubprocessClusterParams;

pub fn subprocess_cluster_lifecycle_in_dedicated_runtime() -> Result<()> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(async {
        let cluster = start_subprocess_cluster(SubprocessClusterParams {
            agents: AgentConfig::uniform(1, 4),
            wait_for_slots_ready: false,
            ..SubprocessClusterParams::default()
        })
        .await?;

        cluster.shutdown().await
    })
}
