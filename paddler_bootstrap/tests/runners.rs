use std::net::SocketAddr;
use std::net::TcpListener;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::bootstrap_agent_params::BootstrapAgentParams;
use paddler_bootstrap::bootstrap_balancer_params::BootstrapBalancerParams;
use paddler_bootstrap::cluster_runner::ClusterRunner;
use paddler_bootstrap::cluster_runner::ClusterRunnerParams;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

fn pick_free_loopback_addr() -> Result<SocketAddr> {
    let probe =
        TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral loopback port")?;
    let addr = probe
        .local_addr()
        .context("failed to read ephemeral local_addr")?;

    drop(probe);

    Ok(addr)
}

async fn wait_until_bound(addr: SocketAddr) -> Result<()> {
    loop {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

fn make_cluster_bootstrap_params(
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
) -> BootstrapBalancerParams {
    BootstrapBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        inference_service_configuration: InferenceServiceConfiguration {
            addr: inference_addr,
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_secs(30),
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: management_addr,
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: 30,
        openai_service_configuration: None,
        state_database_type: StateDatabaseType::Memory,
        statsd_prefix: "paddler_bootstrap_test_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    }
}

fn make_agent_bootstrap_params(management_addr: SocketAddr) -> BootstrapAgentParams {
    BootstrapAgentParams {
        agent_name: Some("test-agent".to_owned()),
        management_address: management_addr.to_string(),
        slots: 1,
    }
}

#[tokio::test]
async fn cluster_runner_exits_when_dropped() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let runner = ClusterRunner::start(ClusterRunnerParams {
        bootstrap_params: make_cluster_bootstrap_params(management_addr, inference_addr),
        initial_desired_state: BalancerDesiredState::default(),
        parent_shutdown: None,
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after cluster runner drop")?;
    TcpListener::bind(inference_addr)
        .context("inference port is still held after cluster runner drop")?;

    Ok(())
}

#[tokio::test]
async fn cluster_runner_exits_on_explicit_cancel() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let mut runner = ClusterRunner::start(ClusterRunnerParams {
        bootstrap_params: make_cluster_bootstrap_params(management_addr, inference_addr),
        initial_desired_state: BalancerDesiredState::default(),
        parent_shutdown: None,
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    runner.cancel();

    let _ = runner.take_initial_bundle_rx();
    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after explicit cancel")?;

    Ok(())
}

#[tokio::test]
async fn cluster_runner_cancels_from_parent_token() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let parent = CancellationToken::new();

    let runner = ClusterRunner::start(ClusterRunnerParams {
        bootstrap_params: make_cluster_bootstrap_params(management_addr, inference_addr),
        initial_desired_state: BalancerDesiredState::default(),
        parent_shutdown: Some(parent.clone()),
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    parent.cancel();
    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after parent cancel")?;

    Ok(())
}

#[tokio::test]
async fn agent_runner_exits_when_dropped() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;

    let mut runner = AgentRunner::start(AgentRunnerParams {
        bootstrap_params: make_agent_bootstrap_params(management_addr),
        parent_shutdown: None,
    });

    let status_rx = runner
        .take_initial_status_rx()
        .ok_or_else(|| anyhow!("AgentRunner did not expose initial_status_rx"))?;
    let _status = status_rx
        .await
        .map_err(|error| anyhow!("agent bootstrap never published status: {error}"))?;

    drop(runner);

    Ok(())
}

#[tokio::test]
async fn agent_runner_cancels_from_parent_token() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;

    let parent = CancellationToken::new();

    let mut runner = AgentRunner::start(AgentRunnerParams {
        bootstrap_params: make_agent_bootstrap_params(management_addr),
        parent_shutdown: Some(parent.clone()),
    });

    let status_rx = runner
        .take_initial_status_rx()
        .ok_or_else(|| anyhow!("AgentRunner did not expose initial_status_rx"))?;
    let _status = status_rx
        .await
        .map_err(|error| anyhow!("agent bootstrap never published status: {error}"))?;

    parent.cancel();
    drop(runner);

    Ok(())
}
