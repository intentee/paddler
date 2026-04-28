use anyhow::Context as _;
use anyhow::Result;
use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_completion::ClusterCompletion;
use crate::cluster_handle::ClusterHandle;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::in_process_cluster_params::InProcessClusterParams;
use crate::wait_until_healthy::wait_until_healthy;

pub struct InProcessCluster;

impl InProcessCluster {
    pub async fn start(
        InProcessClusterParams {
            agent_count,
            agent_name_prefix,
            buffered_request_timeout,
            desired_state,
            inference_cors_allowed_hosts,
            inference_item_timeout,
            management_cors_allowed_hosts,
            max_buffered_requests,
            slots_per_agent,
            wait_for_slots_ready,
        }: InProcessClusterParams,
    ) -> Result<ClusterHandle> {
        let addresses = BalancerAddresses::pick()?;
        let cancel_token = CancellationToken::new();

        let balancer = BalancerRunner::start(BalancerRunnerParams {
            buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: addresses.inference,
                cors_allowed_hosts: inference_cors_allowed_hosts,
                inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: addresses.management,
                cors_allowed_hosts: management_cors_allowed_hosts,
            },
            max_buffered_requests,
            openai_service_configuration: Some(OpenAIServiceConfiguration {
                addr: addresses.compat_openai,
            }),
            parent_shutdown: Some(cancel_token.clone()),
            state_database_type: StateDatabaseType::Memory(Box::new(desired_state.clone())),
            statsd_prefix: "paddler_tests_".to_owned(),
            statsd_service_configuration: None,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: None,
        })
        .await
        .context("failed to start in-process BalancerRunner")?;

        let management_base_url = addresses.management_base_url()?;
        let inference_base_url = addresses.inference_base_url()?;

        wait_until_healthy(&management_base_url, "health")
            .await
            .context("balancer did not become healthy")?;

        let paddler_client = PaddlerClient::new(inference_base_url, management_base_url, 1);

        paddler_client
            .management()
            .put_balancer_desired_state(&desired_state)
            .await
            .map_err(anyhow::Error::new)
            .context("failed to PUT balancer desired state")?;

        let mut agents_watcher = AgentsStreamWatcher::connect(&paddler_client.management()).await?;
        let buffered_requests_watcher =
            BufferedRequestsStreamWatcher::connect(&paddler_client.management()).await?;

        let mut agent_runners: Vec<AgentRunner> = Vec::with_capacity(agent_count);

        for agent_index in 0..agent_count {
            let agent_name = format!("{agent_name_prefix}-{agent_index}");

            let agent_runner = AgentRunner::start(AgentRunnerParams {
                agent_name: Some(agent_name),
                management_address: addresses.management.to_string(),
                parent_shutdown: Some(cancel_token.clone()),
                slots: slots_per_agent,
            });

            agent_runners.push(agent_runner);
        }

        let registered_snapshot = agents_watcher
            .until(move |snapshot| snapshot.agents.len() >= agent_count)
            .await
            .context("not all requested agents registered")?;

        let agent_ids: Vec<String> = registered_snapshot
            .agents
            .iter()
            .map(|agent| agent.id.clone())
            .collect();

        if wait_for_slots_ready {
            agents_watcher
                .until(move |snapshot| {
                    snapshot.agents.len() >= agent_count
                        && snapshot
                            .agents
                            .iter()
                            .all(|agent| agent.slots_total >= slots_per_agent)
                })
                .await
                .context("agents did not reach the requested slot count")?;
        }

        Ok(ClusterHandle::new(ClusterHandleParams {
            addresses,
            agent_ids,
            agents: agents_watcher,
            buffered_requests: buffered_requests_watcher,
            cancel_token,
            completion: ClusterCompletion::InProcess {
                agents: agent_runners,
                balancer,
            },
            paddler_client,
        }))
    }
}
