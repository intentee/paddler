#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_test_fixture::http_response_spec::HttpResponseSpec;
use paddler_test_fixture::local_http_fixture::LocalHttpFixture;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_does_not_exist_at_url() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::status(404, "Not Found")).await?;
    let model_url = format!("{}/missing.gguf", fixture.base_url());

    let mut cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::Url(UrlModelReference {
                    url: model_url.clone(),
                }),
                multimodal_projection: AgentDesiredModel::None,
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

    let snapshot = cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::ModelDoesNotExistAtUrl(_)))
            })
        })
        .await
        .context("balancer should report ModelDoesNotExistAtUrl for 404 URL")?;

    let saw_expected_url = snapshot.agents.iter().any(|agent| {
        agent.issues.iter().any(|issue| {
            matches!(issue, AgentIssue::ModelDoesNotExistAtUrl(model_path)
                if model_path.model_path == model_url)
        })
    });

    assert!(
        saw_expected_url,
        "ModelDoesNotExistAtUrl should reference the configured URL"
    );

    cluster.shutdown().await?;

    Ok(())
}
