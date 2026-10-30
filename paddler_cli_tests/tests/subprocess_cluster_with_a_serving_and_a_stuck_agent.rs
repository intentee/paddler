#![cfg(feature = "tests_that_use_llms")]

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

pub const SERVING_AGENT_NAME: &str = "serving-agent";
pub const SERVING_AGENT_SLOTS: u16 = 1;

pub async fn subprocess_cluster_with_a_serving_and_a_stuck_agent() -> Cluster {
    let qwen3_desired_state = qwen3_0_6b().into_desired_state();
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model_runtime_parameters: ModelRuntimeParameters {
                    n_batch: BatchSize::try_from(u32::from(SERVING_AGENT_SLOTS))
                        .expect("a batch as large as the serving agent's slots is valid"),
                    ..qwen3_desired_state.model_runtime_parameters
                },
                ..qwen3_desired_state
            })),
            ..ClusterParams::without_agents()
        },
    )
    .await
    .expect("the balancer must start");

    cluster
        .register_agents(
            &[AgentConfig {
                name: "stuck-agent".to_owned(),
                slot_count: SERVING_AGENT_SLOTS + 1,
            }],
            false,
        )
        .await
        .expect("the stuck agent must register");
    cluster
        .register_agents(
            &[AgentConfig {
                name: SERVING_AGENT_NAME.to_owned(),
                slot_count: SERVING_AGENT_SLOTS,
            }],
            true,
        )
        .await
        .expect("the serving agent must register and serve");
    cluster
        .wait_for_first_agent_issue(|issue| matches!(issue, AgentIssue::SlotCannotStart(_)))
        .await
        .expect("the stuck agent must report that its slots cannot start in a smaller batch");

    cluster
}
