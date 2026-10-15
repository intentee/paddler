#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::json;

use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_questions_in_their_order() {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("a decision cluster must start");

    let response = cluster
        .typesafe_system_one(
            "typesafe-request",
            &json!({
                "model": "kev-latest",
                "state": "The invoice was paid on time.",
                "questions": {
                    "tone": {"type": "choice", "criteria": {"calm": null, "angry": "Shouting"}},
                    "paid": {"type": "noul", "instructions": "Was it paid?"},
                },
            }),
        )
        .await
        .expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.headers.get(TypeSafeHeader::REQUEST_ID),
        Some(
            &"typesafe-request"
                .parse()
                .expect("the header value is valid")
        )
    );

    let answers = response.body["answers"]
        .as_object()
        .expect("the answers must be an object");

    assert_eq!(answers.keys().collect::<Vec<_>>(), vec!["tone", "paid"]);
    assert_eq!(answers["tone"]["type"], "choice");
    assert_eq!(
        answers["tone"]["probabilities"]
            .as_object()
            .expect("the choice probabilities must be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["calm", "angry"]
    );
    assert_eq!(answers["paid"]["type"], "noul");
    assert_eq!(response.body["model"], "kev-latest");
    assert_eq!(response.body["usage"]["output_tokens"], 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
