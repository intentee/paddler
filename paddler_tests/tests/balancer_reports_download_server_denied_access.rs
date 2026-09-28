use http::StatusCode;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_download_server_denied_access() {
    let fixture = LocalHttpFixture::start(FixtureResponse::Status(StatusCode::FORBIDDEN))
        .await
        .expect("the local HTTP fixture must start");
    let model_url = fixture.url("/private.gguf");
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Url(UrlModelReference {
            url: model_url.clone(),
        }),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::DownloadServerDeniedAccess(model_path) if model_path.model_path == model_url)
        })
        .await
        .expect("the agent must report DownloadServerDeniedAccess for the configured URL");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
