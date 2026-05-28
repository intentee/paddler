#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::agent_issue::AgentIssue;
use paddler::balancer_desired_state::BalancerDesiredState;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_excessive_slots() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);

    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 257,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
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
        cluster.agents.until(|snapshot| {
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

    let slot_cannot_start_count = snapshot
        .agents
        .iter()
        .flat_map(|agent| agent.issues.iter())
        .filter(|issue| matches!(issue, AgentIssue::SlotCannotStart(params) if !params.error.is_empty()))
        .count();

    assert!(
        slot_cannot_start_count > 0,
        "expected at least one SlotCannotStart issue with non-empty error"
    );

    cluster.shutdown().await?;

    Ok(())
}
