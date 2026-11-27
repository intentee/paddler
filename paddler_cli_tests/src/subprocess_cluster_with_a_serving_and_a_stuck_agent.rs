use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

use crate::start_subprocess_cluster::start_subprocess_cluster;

const SERVING_AGENT_SLOTS: u16 = 1;

const STUCK_AGENT_SLOTS: u16 = SERVING_AGENT_SLOTS + 1;

fn desired_state_batching_the_serving_agent_slots() -> Result<BalancerDesiredState> {
    let qwen3_desired_state = qwen3_0_6b().into_desired_state();

    BatchSize::try_from(u32::from(SERVING_AGENT_SLOTS))
        .context("a batch as large as the serving agent's slots is invalid")
        .map(|n_batch| BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                n_batch: ModelDefaultOr::Explicit(n_batch),
                ..qwen3_desired_state.model_runtime_parameters
            },
            ..qwen3_desired_state
        })
}

async fn with_registered_agent(
    mut cluster: Cluster,
    agent_config: AgentConfig,
    wait_for_slots_ready: bool,
) -> Result<Cluster> {
    cluster
        .register_agents(&[agent_config], wait_for_slots_ready)
        .await
        .map(|()| cluster)
}

async fn with_the_stuck_agent_reporting_its_slots(mut cluster: Cluster) -> Result<Cluster> {
    cluster
        .wait_for_first_agent_issue(|issue| matches!(issue, AgentIssue::SlotCannotStart(_)))
        .await
        .map(|_reporting_pool| cluster)
        .map_err(Error::from)
}

pub struct SubprocessClusterWithAServingAndAStuckAgent {
    pub cluster: Cluster,
    pub serving_agent: AgentConfig,
}

impl SubprocessClusterWithAServingAndAStuckAgent {
    /// # Errors
    ///
    /// Fails when the balancer does not start, when either agent does not register, when the serving agent does not serve, and when the stuck agent does not report that its slots cannot start in a smaller batch.
    pub async fn start(binary_path: &str) -> Result<Self> {
        let serving_agent = AgentConfig::single(SERVING_AGENT_SLOTS);

        let started_cluster: Result<Cluster> =
            ready(desired_state_batching_the_serving_agent_slots())
                .and_then(|desired_state| {
                    start_subprocess_cluster(
                        binary_path,
                        ClusterParams {
                            desired_state: ClusterDesiredState::Apply(Box::new(desired_state)),
                            ..ClusterParams::without_agents()
                        },
                    )
                })
                .and_then(|cluster| {
                    with_registered_agent(
                        cluster,
                        AgentConfig {
                            name: "stuck-agent".to_owned(),
                            slot_count: STUCK_AGENT_SLOTS,
                        },
                        false,
                    )
                })
                .and_then(|cluster| with_registered_agent(cluster, serving_agent.clone(), true))
                .and_then(with_the_stuck_agent_reporting_its_slots)
                .await;

        started_cluster.map(|cluster| Self {
            cluster,
            serving_agent,
        })
    }
}
