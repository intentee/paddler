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
    pub fn with_service_config(self, service_config: BalancerServiceConfig) -> Self {
        Self {
            service_config,
            ..self
        }
    }
}

#[async_trait]
impl ClusterBackend for SubprocessClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend> {
        let addresses = BalancerAddresses::pick().await?;
        let management_addr = addresses.management;

        let mut balancer_command = paddler_command(&self.binary_path);

        balancer_command
            .arg("balancer")
            .arg("--inference-addr")
            .arg(addresses.inference.to_string())
            .arg("--management-addr")
            .arg(addresses.management.to_string())
            .arg("--compat-openai-addr")
            .arg(addresses.compat_openai.to_string())
            .args(self.service_config.command_args())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

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
