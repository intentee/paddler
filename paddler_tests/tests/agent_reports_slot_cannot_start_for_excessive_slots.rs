#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_excessive_slots() {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 257,
        }],
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            ..BalancerDesiredState::default()
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");

    timeout(
        Duration::from_secs(10),
        cluster.wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::SlotCannotStart(slot_cannot_start) if !slot_cannot_start.error.is_empty())
        }),
    )
    .await
    .expect("the agent must report SlotCannotStart within 10 seconds")
    .expect("the agent must report SlotCannotStart with the underlying error");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
