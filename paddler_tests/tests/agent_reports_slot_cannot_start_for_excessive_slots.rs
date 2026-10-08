#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_excessive_slots() {
    let ModelCard { reference } = qwen3_0_6b();
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 257,
        }],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                n_gpu_layers: ALL_GPU_LAYERS,
                ..ModelRuntimeParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::SlotCannotStart(slot_cannot_start) if !slot_cannot_start.error.is_empty())
        })
        .await
        .expect("the agent must report SlotCannotStart with the underlying error");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
