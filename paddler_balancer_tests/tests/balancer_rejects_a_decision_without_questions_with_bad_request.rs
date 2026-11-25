use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use reqwest::StatusCode;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_messaging::constants::API_PATH_DECIDE;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_a_decision_without_questions_with_bad_request() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams::default())
            .await
            .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;

    let decide_url = cluster
        .balancer
        .inference_base_url
        .join(API_PATH_DECIDE)
        .expect("the decide path must extend the inference address");
    let rejection = cluster
        .decide(
            CancellationToken::new(),
            &DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::new(),
                state: json!("state"),
            },
        )
        .await
        .expect_err("a decision without questions must be rejected");

    assert!(matches!(
        rejection.downcast_ref::<ClientError>(),
        Some(ClientError::UnexpectedResponseStatus { status, url, .. })
            if *status == StatusCode::BAD_REQUEST && url == decide_url.as_str()
    ));

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
