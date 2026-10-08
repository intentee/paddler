use reqwest::StatusCode;
use serde_json::json;
use tokio::join;

use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::serving_agent_status::serving_agent_status;
use paddler_tests::start_cluster::start_cluster;

const RAW_AGENT_ID: &str = "raw-decision-agent";

fn answer(id: &str, probabilities: Vec<f32>) -> AgentJsonRpcResponse {
    AgentJsonRpcResponse::Decision(DecisionResult::QuestionAnswered(DecisionAnswer {
        id: id.to_owned(),
        probabilities,
    }))
}

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_questions_in_their_order() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Decision))
        .await
        .expect("a decision balancer without agents must start");
    let mut raw_agent_socket =
        RawAgentSocket::connect(cluster.balancer.addresses.management, RAW_AGENT_ID)
            .await
            .expect("the raw agent must reach the agent socket");

    raw_agent_socket
        .register_with_status(serving_agent_status(InferenceMode::Decision))
        .await
        .expect("the raw agent must register");

    let system_one_request = json!({
        "model": "kev-latest",
        "state": "The invoice was paid on time.",
        "questions": {
            "tone": {"type": "choice", "criteria": {"calm": null, "angry": "Shouting"}},
            "paid": {"type": "noul", "instructions": "Was it paid?"},
        },
    });
    let (response, _raw_agent_socket) = join!(
        cluster.typesafe_system_one("typesafe-request", &system_one_request),
        async move {
            let request_envelope = raw_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the decision to the agent");
            let AgentJsonRpcRequest::Decide(decide_params) = request_envelope.request else {
                panic!("the balancer must forward a decision");
            };

            assert_eq!(
                decide_params
                    .leading_questions
                    .iter()
                    .chain([&decide_params.last_question])
                    .map(|question| (question.id.as_str(), question.options.clone()))
                    .collect::<Vec<_>>(),
                vec![
                    (
                        "tone",
                        vec!["calm".to_owned(), "angry: Shouting".to_owned()]
                    ),
                    ("paid", vec!["no".to_owned(), "yes".to_owned()]),
                ]
            );

            for response in [
                answer("tone", vec![0.25, 0.75]),
                answer("paid", vec![0.1, 0.9]),
                AgentJsonRpcResponse::Decision(DecisionResult::Done(DecisionSummary {
                    input_tokens: 42,
                    processing_milliseconds: 7,
                })),
            ] {
                raw_agent_socket
                    .send_response(ResponseEnvelope {
                        generated_by: None,
                        request_id: request_envelope.id.clone(),
                        response,
                    })
                    .await
                    .expect("the raw agent must answer");
            }

            raw_agent_socket
        }
    );
    let response = response.expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.headers.get(TypeSafeHeader::REQUEST_ID),
        Some(
            &"typesafe-request"
                .parse()
                .expect("the header value is valid")
        )
    );

    assert_eq!(
        response.body,
        json!({
            "model": "kev-latest",
            "answers": {
                "tone": {
                    "type": "choice",
                    "choice": "angry",
                    "confidence": 0.5,
                    "probabilities": {"calm": 0.25, "angry": 0.75},
                },
                "paid": {"type": "noul", "noul": 0.9},
            },
            "usage": {"input_tokens": 42, "output_tokens": 0},
            "latency_ms": 7,
        })
    );
    assert_eq!(
        response.body["answers"]
            .as_object()
            .expect("the answers must be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["tone", "paid"]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
