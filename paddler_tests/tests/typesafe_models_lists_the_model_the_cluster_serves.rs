use reqwest::StatusCode;
use serde_json::json;

use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::url_model_reference::UrlModelReference;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_lists_the_model_the_cluster_serves() {
    for (model, expected_names) in [
        (AgentDesiredModel::None, Vec::<&str>::new()),
        (
            AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename: "kev-0.8b.gguf".to_owned(),
                repo_id: "intentee/kev".to_owned(),
                revision: "main".to_owned(),
            }),
            vec!["intentee/kev/main/kev-0.8b.gguf"],
        ),
        (
            AgentDesiredModel::LocalToAgent("/models/kev-0.8b.gguf".to_owned()),
            vec!["/models/kev-0.8b.gguf"],
        ),
        (
            AgentDesiredModel::Url(UrlModelReference {
                url: "https://example.com/kev-0.8b.gguf".to_owned(),
            }),
            vec!["https://example.com/kev-0.8b.gguf"],
        ),
    ] {
        let cluster = start_cluster(ClusterParams {
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model,
                inference_mode: InferenceMode::Decision,
                ..BalancerDesiredState::default()
            })),
            ..cluster_without_agents_serving(InferenceMode::Decision)
        })
        .await
        .expect("a decision balancer without agents must start");

        let response = cluster
            .typesafe_models()
            .await
            .expect("the models request must be answered");

        assert_eq!(response.status, StatusCode::OK);
        assert!(
            response
                .headers
                .get(TypeSafeHeader::REQUEST_ID)
                .is_some_and(|request_id| !request_id.is_empty())
        );
        assert_eq!(
            response.body,
            json!({
                "models": expected_names
                    .iter()
                    .map(|name| json!({
                        "name": name,
                        "description": format!("Decision model {name} served by Paddler"),
                        "release_date": "unknown",
                    }))
                    .collect::<Vec<_>>(),
            })
        );

        cluster
            .shutdown()
            .await
            .expect("the cluster must shut down cleanly");
    }
}
