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
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_agent_params::BootstrapAgentParams;
use paddler_bootstrap::bootstrap_balancer_params::BootstrapBalancerParams;
use paddler_client::PaddlerClient;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::net::TcpStream;
use url::Url;

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

async fn wait_for_agent_registered(
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
) -> Result<()> {
    let management_url = Url::parse(&format!("http://{management_addr}"))?;
    let inference_url = Url::parse(&format!("http://{inference_addr}"))?;
    let client = PaddlerClient::new(inference_url, management_url, 1);

    loop {
        let agents = client
            .management()
            .get_agents()
            .await
            .context("failed to read agents from management API")?;

        if !agents.agents.is_empty() {
            return Ok(());
        }

        tokio::task::yield_now().await;
    }
}

fn make_balancer_bootstrap_params(
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
        statsd_prefix: "paddler_gui_shape_test_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    }
}

fn make_agent_bootstrap_params(management_addr: SocketAddr) -> BootstrapAgentParams {
    BootstrapAgentParams {
        agent_name: Some("gui-shape-test-agent".to_owned()),
        management_address: management_addr.to_string(),
        slots: 1,
    }
}

#[tokio::test]
async fn balancer_exits_while_real_agent_is_connected() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let balancer = BalancerRunner::start(BalancerRunnerParams {
        bootstrap_params: make_balancer_bootstrap_params(management_addr, inference_addr),
        initial_desired_state: Some(BalancerDesiredState::default()),
        parent_shutdown: None,
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    let mut agent = AgentRunner::start(AgentRunnerParams {
        bootstrap_params: make_agent_bootstrap_params(management_addr),
        parent_shutdown: None,
    });

    let _status = agent
        .take_initial_status_rx()
        .ok_or_else(|| anyhow!("AgentRunner did not expose initial_status_rx"))?
        .await
        .map_err(|error| anyhow!("agent bootstrap never published status: {error}"))?;

    wait_for_agent_registered(management_addr, inference_addr).await?;

    drop(balancer);
    drop(agent);

    TcpListener::bind(management_addr)
        .context("management port is still held after balancer + agent drop")?;
    TcpListener::bind(inference_addr)
        .context("inference port is still held after balancer + agent drop")?;

    Ok(())
}

#[tokio::test]
async fn agent_exits_while_connected_to_balancer() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let balancer = BalancerRunner::start(BalancerRunnerParams {
        bootstrap_params: make_balancer_bootstrap_params(management_addr, inference_addr),
        initial_desired_state: Some(BalancerDesiredState::default()),
        parent_shutdown: None,
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    let mut agent = AgentRunner::start(AgentRunnerParams {
        bootstrap_params: make_agent_bootstrap_params(management_addr),
        parent_shutdown: None,
    });

    let _status = agent
        .take_initial_status_rx()
        .ok_or_else(|| anyhow!("AgentRunner did not expose initial_status_rx"))?
        .await
        .map_err(|error| anyhow!("agent bootstrap never published status: {error}"))?;

    wait_for_agent_registered(management_addr, inference_addr).await?;

    drop(agent);
    drop(balancer);

    TcpListener::bind(management_addr)
        .context("management port is still held after agent + balancer drop")?;

    Ok(())
}
