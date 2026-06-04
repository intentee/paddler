#![cfg(all(feature = "tests_that_use_llms", feature = "metal"))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::kv_cache_dtype::KvCacheDtype;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_slot_cannot_start_for_metal_quantized_distinct_kv() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        ..InferenceParameters::default()
    };

    inference_parameters.k_cache_dtype = KvCacheDtype::Q80;
    inference_parameters.v_cache_dtype = KvCacheDtype::Q40;

    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
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
        cluster.agents_watcher.until(|snapshot| {
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
