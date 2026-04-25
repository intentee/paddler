#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn gui_balancer_exits_cleanly_while_agent_connected() -> Result<()> {
    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    let inference_addr = cluster.addresses.inference;
    let management_addr = cluster.addresses.management;
    let compat_openai_addr = cluster.addresses.compat_openai;

    cluster.shutdown().await?;

    TcpListener::bind(inference_addr)
        .context("inference port must be released after cluster shutdown")?;
    TcpListener::bind(management_addr)
        .context("management port must be released after cluster shutdown")?;
    TcpListener::bind(compat_openai_addr)
        .context("OpenAI compat port must be released after cluster shutdown")?;

    Ok(())
}
