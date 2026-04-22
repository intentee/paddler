use std::net::SocketAddr;
use std::net::TcpListener;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::bootstrap_balancer_params::BootstrapBalancerParams;
use paddler_bootstrap::bootstrapped_balancer_handle::bootstrap_balancer;
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

#[tokio::test]
async fn balancer_releases_port_after_shutdown() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let shutdown = CancellationToken::new();
    let balancer_shutdown = shutdown.clone();

    let balancer_task = tokio::task::spawn_blocking(move || -> Result<()> {
        actix_web::rt::System::new().block_on(async move {
            let bootstrapped = bootstrap_balancer(BootstrapBalancerParams {
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
                #[cfg(feature = "web_admin_panel")]
                web_admin_panel_service_configuration: None,
            })
            .await?;

            let service_handle =
                actix_web::rt::spawn(bootstrapped.service_manager.run_forever(balancer_shutdown));

            service_handle
                .await
                .map_err(|error| anyhow!("service manager task panicked: {error}"))?
        })
    });

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    shutdown.cancel();

    balancer_task
        .await
        .map_err(|join_error| anyhow!("balancer task panicked: {join_error}"))??;

    TcpListener::bind(management_addr)
        .context("management port is still held after balancer shutdown")?;
    TcpListener::bind(inference_addr)
        .context("inference port is still held after balancer shutdown")?;

    Ok(())
}
