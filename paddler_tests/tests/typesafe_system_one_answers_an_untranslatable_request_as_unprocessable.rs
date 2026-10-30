use reqwest::StatusCode;
use serde_json::json;

use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
use paddler_test_cluster_harness::system_one_request::system_one_request;
use paddler_tests::cluster_without_agents_serving_decisions::cluster_without_agents_serving_decisions;
use paddler_tests::start_cluster::start_cluster;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_an_untranslatable_request_as_unprocessable() {
    let cluster = start_cluster(cluster_without_agents_serving_decisions())
        .await
        .expect("a decision balancer without agents must start");

    let without_questions = cluster
        .typesafe_system_one(
            "untranslatable-request",
            &system_one_request(&json!("state"), &json!({})),
        )
        .await
        .expect("the request must be answered");

    assert_eq!(without_questions.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        without_questions.body,
        json!({"detail": TypeSafeTranslationError::DecisionInvalid {
            source: RequestParamsValidationError::DecisionWithoutQuestions,
        }
        .to_string()})
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
