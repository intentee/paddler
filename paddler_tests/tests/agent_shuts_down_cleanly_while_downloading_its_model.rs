use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_downloading_its_model() {
    let fixture = LocalHttpFixture::start(FixtureResponse::StallBeforeHeaders)
        .await
        .expect("the local HTTP fixture must start");
    let cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Uri(fixture.url("/stalled.gguf")),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    fixture
        .wait_for_requests(1)
        .await
        .expect("the agent must start downloading its model");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly while its agent downloads a model");

    fixture
        .stop()
        .await
        .expect("the fixture task must not panic")
        .expect("the fixture must serve every request");
}
