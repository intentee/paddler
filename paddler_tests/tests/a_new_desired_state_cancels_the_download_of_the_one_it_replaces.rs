use std::time::Duration;

use tokio_util::sync::CancellationToken;

use paddler_download_manager::read_timeout::READ_TIMEOUT;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

const BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT: Duration =
    Duration::from_secs(READ_TIMEOUT.as_secs() / 2);

#[tokio::test(flavor = "multi_thread")]
async fn a_new_desired_state_cancels_the_download_of_the_one_it_replaces() {
    let fixture = LocalHttpFixture::start(FixtureResponse::EndlessTrickle {
        byte_interval: BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT,
    })
    .await
    .expect("the local HTTP fixture must start");
    let cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Uri(fixture.url("/endless.gguf")),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    fixture
        .wait_for_requests(1)
        .await
        .expect("the agent must start downloading the model of the replaced desired state");

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &BalancerDesiredState::default())
        .await
        .expect("the balancer must apply the replacement desired state");

    fixture
        .wait_for_abandoned_trickles(1)
        .await
        .expect("the agent must cancel the download of the replaced desired state");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    fixture
        .stop()
        .await
        .expect("the fixture task must not panic")
        .expect("the fixture must serve every request");
}
