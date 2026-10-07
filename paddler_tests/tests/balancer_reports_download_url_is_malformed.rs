use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_download_url_is_malformed() {
    let model_url = "not a valid url".to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Url(UrlModelReference {
            url: model_url.clone(),
        }),
        ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::DownloadUrlIsMalformed(model_path) if model_path.model_path == model_url)
        })
        .await
        .expect("the agent must report DownloadUrlIsMalformed for the configured URL");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
