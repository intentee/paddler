use std::str::FromStr as _;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_cluster::balancer_addresses::BalancerAddresses;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster_backend::ClusterBackend;
use paddler_cluster::provisioned_backend::ProvisionedBackend;
use paddler_cluster::running_balancer::RunningBalancer;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::in_process_agent_spawner::InProcessAgentSpawner;
use crate::in_process_balancer::InProcessBalancer;

#[derive(Default)]
pub struct InProcessClusterBackend {
    service_config: BalancerServiceConfig,
}

impl InProcessClusterBackend {
    #[must_use]
    pub fn with_service_config(self, service_config: BalancerServiceConfig) -> Self {
        Self { service_config }
    }
}

#[async_trait]
impl ClusterBackend for InProcessClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend> {
        let BalancerServiceConfig {
            buffered_request_timeout,
            inference_cors_allowed_hosts,
            inference_item_timeout,
            management_cors_allowed_hosts,
            max_buffered_requests,
            state_database_url,
        } = &self.service_config;

        log::set_max_level(log::LevelFilter::Trace);

        let addresses = BalancerAddresses::pick().await?;
        let management_address = addresses.management.to_string();
        let state_database_type = StateDatabaseType::from_str(state_database_url)
            .context("failed to parse state_database_url")?;

        let balancer_runner = BalancerRunner::start(BalancerRunnerParams {
            buffered_request_timeout: *buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: addresses.inference,
                cors_allowed_hosts: inference_cors_allowed_hosts.clone(),
                inference_item_timeout: *inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: addresses.management,
                cors_allowed_hosts: management_cors_allowed_hosts.clone(),
            },
            max_buffered_requests: *max_buffered_requests,
            openai_service_configuration: Some(OpenAIServiceConfiguration {
                addr: addresses.compat_openai,
            }),
            cancellation_token: CancellationToken::new(),
            shutdown_options: ServiceShutdownOptions::default(),
            state_database_type,
            statsd_prefix: "paddler_tests_".to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        })
        .await
        .context("failed to start in-process BalancerRunner")?;

        let running_balancer =
            RunningBalancer::new(addresses, Box::new(InProcessBalancer::new(balancer_runner)));

        Ok(ProvisionedBackend {
            agent_spawner: Box::new(InProcessAgentSpawner::new(management_address)),
            running_balancer,
        })
    }
}
