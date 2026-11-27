use std::str::FromStr as _;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer_state_database::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_bootstrap_config::BalancerBootstrapConfig;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;

use crate::agent_config::AgentConfig;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::ephemeral_loopback_addr::ephemeral_loopback_addr;
use crate::longer_than_any_test_run::longer_than_any_test_run;
use crate::unexpiring_shutdown_options::unexpiring_shutdown_options;

pub struct ClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub desired_state: ClusterDesiredState,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: u64,
    pub state_database_url: String,
    pub wait_for_slots_ready: bool,
}

impl ClusterParams {
    #[must_use]
    pub fn without_agents() -> Self {
        Self {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..Self::default()
        }
    }

    /// # Errors
    ///
    /// Returns an error when the state database URL of the cluster parameters cannot be parsed.
    pub fn in_process_balancer_runner_params(&self) -> Result<BalancerRunnerParams> {
        let Self {
            buffered_request_timeout,
            inference_cors_allowed_hosts,
            inference_item_timeout,
            management_cors_allowed_hosts,
            max_buffered_requests,
            state_database_url,
            ..
        } = self;

        Ok(BalancerRunnerParams {
            bootstrap_config: BalancerBootstrapConfig {
                buffered_request_timeout: *buffered_request_timeout,
                inference_service_configuration: InferenceServiceConfiguration {
                    addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
                    cors_allowed_hosts: inference_cors_allowed_hosts.clone(),
                    inference_item_timeout: *inference_item_timeout,
                },
                management_service_configuration: ManagementServiceConfiguration {
                    addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
                    cors_allowed_hosts: management_cors_allowed_hosts.clone(),
                },
                max_buffered_requests: *max_buffered_requests,
                openai_service_configuration: Some(CompatibilityServiceConfiguration {
                    addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
                }),
                state_database_type: StateDatabaseType::from_str(state_database_url)
                    .context("failed to parse state_database_url")?,
                statsd_prefix: "paddler_cluster_harness_".to_owned(),
                statsd_service_configuration: None,
                typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
                    addr: ResolvedSocketAddr::from(ephemeral_loopback_addr()),
                }),
                #[cfg(feature = "web_admin_panel")]
                web_admin_panel_service_configuration: None,
            },
            cancellation_token: CancellationToken::new(),
            shutdown_options: unexpiring_shutdown_options(),
        })
    }
}

impl Default for ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            buffered_request_timeout: longer_than_any_test_run(),
            desired_state: ClusterDesiredState::Apply(Box::default()),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: longer_than_any_test_run(),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            state_database_url: "memory://".to_owned(),
            wait_for_slots_ready: true,
        }
    }
}
