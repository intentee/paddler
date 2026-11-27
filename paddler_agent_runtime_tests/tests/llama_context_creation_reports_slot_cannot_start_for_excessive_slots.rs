#![cfg(feature = "tests_that_use_llms")]

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_model_card_fixture::model_card::ModelCard;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn llama_context_creation_reports_slot_cannot_start_for_excessive_slots() {
    let ModelCard { reference } = qwen3_0_6b();
    let mut cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 257,
        }],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                n_gpu_layers: ModelDefaultOr::Explicit(ALL_GPU_LAYERS),
                ..DesiredModelRuntimeParameters::default()
            },
            model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::SlotCannotStart(SlotCannotStartParams {
                    inference_capability: InferenceCapability::TextGeneration,
                    slot_index: 0,
                    ..
                })
            )
        })
        .await
        .expect("the agent must report that its first text generation slot cannot start");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
