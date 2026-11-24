#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::decision_desired_state::DecisionDesiredState;
use paddler_tests::start_cluster::start_cluster;

const SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_a_batch_smaller_than_its_slots() {
    for desired_state in [
        DecisionDesiredState::default().into_balancer_desired_state(),
        nomic_embed_text_v1_5()
            .into_desired_state_with_embedding_parameters(EmbeddingParameters::default()),
    ] {
        let mut cluster = start_cluster(ClusterParams {
            agents: vec![AgentConfig::single(SLOTS)],
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model_runtime_parameters: ModelRuntimeParameters {
                    n_batch: BatchSize::try_from(u32::from(SLOTS) - 1)
                        .expect("a batch of one token is valid"),
                    n_gpu_layers: ALL_GPU_LAYERS,
                    ..ModelRuntimeParameters::default()
                },
                ..desired_state
            })),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        })
        .await
        .expect("the cluster must start");

        cluster
            .wait_for_first_agent_issue(|issue| {
                matches!(
                    issue,
                    AgentIssue::SlotCannotStart(slot_cannot_start)
                        if slot_cannot_start.slot_index == u32::from(SLOTS) - 1
                )
            })
            .await
            .expect("the agent must report that its slots cannot start");

        cluster
            .shutdown()
            .await
            .expect("the cluster must shut down cleanly");
    }
}
