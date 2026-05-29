#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::agent_issue::AgentIssue;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_mmproj_cannot_be_loaded_for_invalid_file() -> Result<()> {
    let invalid_mmproj_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/invalid_mmproj.gguf"
    );

    let ModelCard { reference, .. } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::LocalToAgent(invalid_mmproj_path.to_owned()),
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let watch_agent_id = agent_id.clone();
    let expected_path = invalid_mmproj_path.to_owned();

    cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == watch_agent_id
                    && agent.issues.iter().any(|issue| {
                        matches!(issue, AgentIssue::MultimodalProjectionCannotBeLoaded(model_path)
                            if model_path.model_path == expected_path)
                    })
            })
        })
        .await
        .context("balancer should report MultimodalProjectionCannotBeLoaded for invalid mmproj")?;

    cluster.shutdown().await?;

    Ok(())
}
