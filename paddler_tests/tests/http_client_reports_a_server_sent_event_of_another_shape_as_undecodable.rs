use futures_util::StreamExt as _;
use serde_json::from_str;
use serde_json::to_string;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error;
use paddler_client::http_client::HttpClient;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn http_client_reports_a_server_sent_event_of_another_shape_as_undecodable() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer without agents must start");
    let mut events = HttpClient::new(
        cluster
            .balancer
            .management_base_url()
            .expect("the balancer must expose its management address"),
    )
    .get_sse_json::<String>(CancellationToken::new(), ApiPath::AGENTS_STREAM)
    .await
    .expect("the agents event stream must open");
    let empty_pool_snapshot = to_string(&AgentControllerPoolSnapshot { agents: Vec::new() })
        .expect("an empty agents snapshot must serialize");

    assert_eq!(
        events
            .next()
            .await
            .and_then(Result::err)
            .as_ref()
            .map(ToString::to_string),
        Some(
            Error::Json(
                from_str::<String>(&empty_pool_snapshot)
                    .expect_err("an agents snapshot must not decode as a string")
            )
            .to_string()
        )
    );

    drop(events);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
