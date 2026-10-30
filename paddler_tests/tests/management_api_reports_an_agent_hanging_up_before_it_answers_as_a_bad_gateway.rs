use reqwest::Client;
use reqwest::StatusCode;
use tokio::spawn;

use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

const AGENT_ID: &str = "hanging-up";

#[tokio::test(flavor = "multi_thread")]
async fn management_api_reports_an_agent_hanging_up_before_it_answers_as_a_bad_gateway() {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut hanging_up_agent = UntrustedManagementAgent::connect_registered(
        cluster.balancer.addresses.management,
        AGENT_ID,
        SlotAggregatedStatusSnapshot {
            status: AgentStatus::default(),
            version: 1,
        },
    )
    .await
    .expect("the agent must register");
    cluster
        .wait_for_agent_count(1)
        .await
        .expect("the balancer must register the agent");

    let model_metadata_response = spawn(
        Client::new()
            .get(
                cluster
                    .balancer
                    .management_base_url
                    .join(&ApiPath::agent_model_metadata(AGENT_ID))
                    .expect("the model metadata path must form a URL"),
            )
            .send(),
    );

    hanging_up_agent
        .next_request()
        .await
        .expect("the balancer must ask the agent for its model metadata");
    drop(hanging_up_agent);

    assert_eq!(
        model_metadata_response
            .await
            .expect("the model metadata request must finish")
            .expect("the balancer must answer the model metadata request")
            .status(),
        StatusCode::BAD_GATEWAY
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
