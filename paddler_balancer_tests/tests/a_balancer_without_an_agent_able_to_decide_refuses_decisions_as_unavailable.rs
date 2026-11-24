use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_without_an_agent_able_to_decide_refuses_decisions_as_unavailable() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let decision_rejection = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &RawDecideParams {
                questions: vec![DecisionQuestion {
                    id: "paid".to_owned(),
                    instructions: "Was it paid?".to_owned(),
                    options: vec!["no".to_owned(), "yes".to_owned()],
                }],
                state: "The invoice was paid on time.".to_owned(),
            },
        )
        .await
        .err()
        .expect("a cluster without an agent able to decide must refuse a decision");

    assert!(matches!(
        decision_rejection,
        ClientError::ServiceUnavailable { .. }
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
