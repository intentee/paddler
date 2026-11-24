use reqwest::StatusCode;
use tokio::spawn;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_tests::start_cluster::start_cluster;
use paddler_tests::untrusted_management_agent::UntrustedManagementAgent;

const DECISION_SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_answers_that_do_not_match_its_questions_as_a_bad_gateway() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");
    let mut untruthful_agent = UntrustedManagementAgent::connect_registered(
        cluster.balancer.addresses.management,
        "untruthful",
        SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                desired_slots_total: DECISION_SLOTS,
                runtime: AgentRuntimeStatus::Serving {
                    decisions: AgentDecisionsServing::Served,
                    slots_total: u64::from(DECISION_SLOTS),
                },
                ..AgentStatus::default()
            },
            version: 1,
        },
    )
    .await
    .expect("the untruthful agent must register");

    let typesafe_response =
        spawn(cluster.typesafe_system_one("mismatched-answers", &noul_system_one_request()));
    let decision_request_id = untruthful_agent
        .next_request()
        .await
        .expect("the balancer must dispatch the decision")
        .id;

    for decision_result in [
        DecisionResult::QuestionAnswered(DecisionAnswer {
            id: "unasked".to_owned(),
            probabilities: vec![0.5, 0.5],
        }),
        DecisionResult::Done(DecisionSummary {
            input_tokens: 1,
            processing_milliseconds: 1,
        }),
    ] {
        untruthful_agent
            .respond(
                decision_request_id.clone(),
                AgentJsonRpcResponse::Decision(decision_result),
            )
            .await
            .expect("the decision result must be sent");
    }

    let response = typesafe_response
        .await
        .expect("the TypeSafe request task must not panic")
        .expect("the request must be answered");

    assert_eq!(response.status, StatusCode::BAD_GATEWAY);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
