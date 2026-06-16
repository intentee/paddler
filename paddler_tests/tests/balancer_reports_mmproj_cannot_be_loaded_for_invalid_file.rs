#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_mmproj_cannot_be_loaded_for_invalid_file() -> Result<()> {
    let invalid_mmproj_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/invalid_mmproj.gguf"
    );

    let ModelCard { reference, .. } = qwen3_0_6b();

    let mut cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::LocalToAgent(
                    invalid_mmproj_path.to_owned(),
                ),
                use_chat_template_override: false,
            }),
        },
    )
    .await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have one registered agent")?;

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
