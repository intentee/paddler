#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_excessive_slots_subprocess() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);

    let mut cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 257,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..SubprocessClusterParams::default()
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
