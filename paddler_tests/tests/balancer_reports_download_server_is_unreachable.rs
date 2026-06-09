#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_download_server_is_unreachable() -> Result<()> {
    let model_url = "http://127.0.0.1:1/model.gguf".to_owned();

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::Url(UrlModelReference {
                url: model_url.clone(),
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

    let snapshot = cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::DownloadServerIsUnreachable(_)))
            })
        })
        .await
        .context(
            "balancer should report DownloadServerIsUnreachable when the URL points at a dead port",
        )?;

    let saw_expected_url = snapshot.agents.iter().any(|agent| {
        agent.issues.iter().any(|issue| {
            matches!(issue, AgentIssue::DownloadServerIsUnreachable(model_path)
                if model_path.model_path == model_url)
        })
    });

    assert!(
        saw_expected_url,
        "DownloadServerIsUnreachable should reference the configured URL"
    );

    cluster.shutdown().await?;

    Ok(())
}
