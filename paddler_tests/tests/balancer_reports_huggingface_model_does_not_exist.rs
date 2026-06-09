#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_huggingface_model_does_not_exist() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename: "nonexistent.gguf".to_owned(),
                repo_id: "nonexistent-org/nonexistent-model-gguf".to_owned(),
                revision: "main".to_owned(),
            }),
            multimodal_projection: AgentDesiredModel::None,
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

    cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent.issues.iter().any(|issue| {
                        matches!(
                            issue,
                            AgentIssue::HuggingFaceModelDoesNotExist(_)
                                | AgentIssue::HuggingFacePermissions(_)
                        )
                    })
            })
        })
        .await
        .context("balancer should report a HuggingFace lookup issue for nonexistent repo")?;

    cluster.shutdown().await?;

    Ok(())
}
