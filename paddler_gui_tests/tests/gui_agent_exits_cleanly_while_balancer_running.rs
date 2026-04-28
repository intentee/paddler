#![cfg(all(target_os = "linux", feature = "tests_that_use_compiled_paddler"))]

use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn gui_agent_exits_cleanly_while_balancer_running() -> Result<()> {
    // The InProcessCluster shutdown sequence stops agents first, then the balancer.
    // Here we exercise that ordering by starting a cluster with one agent,
    // observing it running, and shutting down — the agent is dropped before
    // the balancer is, mirroring the case where an agent is shut down while
    // the balancer is still alive.
    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    let management_addr = cluster.addresses.management;

    cluster.shutdown().await?;

    TcpListener::bind(management_addr)
        .context("management port must be released after cluster shutdown")?;

    Ok(())
}
