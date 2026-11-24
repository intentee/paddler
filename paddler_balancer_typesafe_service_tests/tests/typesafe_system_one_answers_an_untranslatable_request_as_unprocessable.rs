use reqwest::StatusCode;
use serde_json::json;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::system_one_request::system_one_request;
use paddler_tests::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_an_untranslatable_request_as_unprocessable() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams::default())
            .await
            .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;

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
        json!({"detail": TypeSafeTranslationError::DecisionWithoutQuestions.to_string()})
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
