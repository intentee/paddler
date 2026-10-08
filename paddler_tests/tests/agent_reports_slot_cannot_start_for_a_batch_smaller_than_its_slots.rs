#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::start_cluster::start_cluster;

const SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_a_batch_smaller_than_its_slots() {
    for desired_state in [
        qwen3_5_0_8b()
            .into_decision_desired_state(pointer_head_fixture(SYNTHETIC_POINTER_HEAD_FIXTURE)),
        nomic_embed_text_v1_5().into_embeddings_desired_state(EmbeddingParameters::default()),
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
                            && !slot_cannot_start.error.is_empty()
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
