#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms",
    feature = "metal"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;
use paddler_tests::test_device::TestDevice;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::kv_cache_dtype::KvCacheDtype;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_metal_quantized_distinct_kv_in_process() -> Result<()>
{
    let device = current_test_device()?;

    if !matches!(device, TestDevice::Metal) {
        return Ok(());
    }

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);

    inference_parameters.k_cache_dtype = KvCacheDtype::Q8_0;
    inference_parameters.v_cache_dtype = KvCacheDtype::Q4_0;

    let mut cluster = start_in_process_cluster(InProcessClusterParams {
        spawn_agent: true,
        slots_per_agent: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
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
