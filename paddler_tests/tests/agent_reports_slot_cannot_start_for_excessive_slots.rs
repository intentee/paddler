#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use llama_cpp_bindings::LlamaContextLoadError;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_model_source::model_source::ModelSource;
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
            model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");

    let expected_issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
        error: AgentRuntimeError::ContextCreationFailed(LlamaContextLoadError::Unconstructible)
            .to_string(),
        inference_capability: InferenceCapability::Embeddings,
        slot_index: 0,
    });
    let snapshot = cluster
        .wait_for_first_agent_issue(|issue| discriminant(issue) == discriminant(&expected_issue))
        .await
        .expect("the agent must report SlotCannotStart with the underlying error");
    let issues = &snapshot.agents[0].status.issues;

    assert!(issues.contains(&expected_issue), "{issues:?}");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
