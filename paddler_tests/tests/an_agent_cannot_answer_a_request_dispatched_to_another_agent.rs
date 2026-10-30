use std::collections::BTreeMap;

use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

const ANSWERING_AGENT_ID: &str = "answering";
const INJECTING_AGENT_ID: &str = "injecting";
const INJECTING_AGENT_DESIRED_SLOTS_AFTER_INJECTION: u16 = 7;

fn model_metadata_named(name: &str) -> ModelMetadata {
    ModelMetadata {
        metadata: BTreeMap::from([("general.name".to_owned(), name.to_owned())]),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_cannot_answer_a_request_dispatched_to_another_agent() {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let management_addr = cluster.balancer.addresses.management;
    let mut answering_agent = UntrustedManagementAgent::connect_registered(
        management_addr,
        ANSWERING_AGENT_ID,
        SlotAggregatedStatusSnapshot::default(),
    )
    .await
    .expect("the answering agent must register");
    let mut injecting_agent = UntrustedManagementAgent::connect_registered(
        management_addr,
        INJECTING_AGENT_ID,
        SlotAggregatedStatusSnapshot::default(),
    )
    .await
    .expect("the injecting agent must register");

    cluster
        .wait_for_agent_count(2)
        .await
        .expect("the balancer must register both agents");

    let client_management = cluster.client_management.clone();
    let model_metadata_answer = spawn(async move {
        client_management
            .get_model_metadata(CancellationToken::new(), ANSWERING_AGENT_ID)
            .await
    });
    let model_metadata_request_id = answering_agent
        .next_request()
        .await
        .expect("the balancer must ask the answering agent for its model metadata")
        .id;

    injecting_agent
        .respond(
            model_metadata_request_id.clone(),
            AgentJsonRpcResponse::ModelMetadata(Some(model_metadata_named("injected"))),
        )
        .await
        .expect("the injected response must be sent");
    injecting_agent
        .report_status(SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                desired_slots_total: INJECTING_AGENT_DESIRED_SLOTS_AFTER_INJECTION,
                ..AgentStatus::default()
            },
            version: 1,
        })
        .await
        .expect("the status report following the injection must be sent");
    cluster
        .agents_watcher
        .until_agent(INJECTING_AGENT_ID, |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == INJECTING_AGENT_ID
                    && agent.status.desired_slots_total
                        == INJECTING_AGENT_DESIRED_SLOTS_AFTER_INJECTION
            })
        })
        .await
        .expect("the balancer must handle everything the injecting agent sent");
    answering_agent
        .respond(
            model_metadata_request_id,
            AgentJsonRpcResponse::ModelMetadata(Some(model_metadata_named("answered"))),
        )
        .await
        .expect("the genuine response must be sent");

    assert_eq!(
        model_metadata_answer
            .await
            .expect("the model metadata request task must not panic")
            .expect("the balancer must answer the model metadata request"),
        Some(model_metadata_named("answered"))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
