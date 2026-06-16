use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use paddler_cluster::balancer_addresses::BalancerAddresses;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster_backend::ClusterBackend;
use paddler_cluster::provisioned_backend::ProvisionedBackend;
use paddler_cluster::running_balancer::RunningBalancer;

use crate::paddler_command::paddler_command;
use crate::subprocess_agent_spawner::SubprocessAgentSpawner;
use crate::subprocess_process::SubprocessProcess;

pub struct SubprocessClusterBackend {
    binary_path: String,
    service_config: BalancerServiceConfig,
}

impl SubprocessClusterBackend {
    #[must_use]
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary_path: binary_path.to_owned(),
            service_config: BalancerServiceConfig::default(),
        }
    }

    #[must_use]
    pub fn with_service_config(binary_path: &str, service_config: BalancerServiceConfig) -> Self {
        Self {
            binary_path: binary_path.to_owned(),
            service_config,
        }
    }
}

#[async_trait]
impl ClusterBackend for SubprocessClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend> {
        let BalancerServiceConfig {
            buffered_request_timeout,
            inference_cors_allowed_hosts,
            inference_item_timeout,
            management_cors_allowed_hosts,
            max_buffered_requests,
            state_database_url,
        } = &self.service_config;

        let addresses = BalancerAddresses::pick().await?;
        let management_addr = addresses.management;
        let compat_openai_addr = addresses
            .compat_openai
            .context("compat-openai address is required for the subprocess balancer")?;

        let mut balancer_command = paddler_command(&self.binary_path);

        balancer_command
            .arg("balancer")
            .arg("--inference-addr")
            .arg(addresses.inference.to_string())
            .arg("--management-addr")
            .arg(addresses.management.to_string())
            .arg("--compat-openai-addr")
            .arg(compat_openai_addr.to_string())
            .arg("--state-database")
            .arg(state_database_url)
            .arg("--max-buffered-requests")
            .arg(max_buffered_requests.to_string())
            .arg("--buffered-request-timeout")
            .arg(buffered_request_timeout.as_millis().to_string())
            .arg("--inference-item-timeout")
            .arg(inference_item_timeout.as_millis().to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        for allowed_host in inference_cors_allowed_hosts {
            balancer_command
                .arg("--inference-cors-allowed-host")
                .arg(allowed_host);
        }

        for allowed_host in management_cors_allowed_hosts {
            balancer_command
                .arg("--management-cors-allowed-host")
                .arg(allowed_host);
        }

        let balancer_subprocess = balancer_command
            .spawn()
            .context("failed to spawn paddler balancer subprocess")?;

        let running_balancer = RunningBalancer::new(
            addresses,
            Box::new(SubprocessProcess::new(balancer_subprocess)),
        );

        Ok(ProvisionedBackend {
            agent_spawner: Box::new(SubprocessAgentSpawner::new(
                self.binary_path.clone(),
                management_addr,
            )),
            running_balancer,
        })
    }
}
