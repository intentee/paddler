#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

const SLOT_COUNT: i32 = 4;
const TOO_SMALL_BATCH: usize = 2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_when_batch_is_smaller_than_slot_count() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: SLOT_COUNT,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                n_batch: TOO_SMALL_BATCH,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let snapshot = tokio::time::timeout(
        Duration::from_secs(10),
        cluster
            .agents_watcher
            .until(ObservationWindow::model_load(), |snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::SlotCannotStart(_)))
                })
            }),
    )
    .await
    .context("agent did not report SlotCannotStart within 10s")??;

    assert!(
        snapshot
            .agents
            .iter()
            .flat_map(|agent| agent.issues.iter())
            .any(|issue| matches!(
                issue,
                AgentIssue::SlotCannotStart(params) if !params.error.is_empty()
            )),
        "a batch too small for the configured slots must be reported, not abort the agent"
    );

    cluster.shutdown().await?;

    Ok(())
}
