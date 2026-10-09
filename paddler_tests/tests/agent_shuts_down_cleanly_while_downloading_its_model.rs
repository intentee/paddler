use tokio::task::yield_now;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_downloading_its_model() {
    let fixture = LocalHttpFixture::start(FixtureResponse::StallBeforeHeaders)
        .await
        .expect("the local HTTP fixture must start");
    let cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Url(UrlModelReference {
            url: fixture.url("/stalled.gguf"),
        }),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    while fixture.request_count() == 0 {
        yield_now().await;
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly while its agent downloads a model");
}
