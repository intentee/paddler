#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn decision_answers_equal_separate_single_question_decisions() {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let decision = sample_decision();

    let joint_answers = cluster
        .decide(CancellationToken::new(), &decision)
        .await
        .expect("the joint decision must be accepted")
        .answers;
    let mut separate_answers = Vec::new();

    for question in &decision.questions {
        separate_answers.extend(
            cluster
                .decide(
                    CancellationToken::new(),
                    &RawDecideParams {
                        questions: vec![question.clone()],
                        state: decision.state.clone(),
                    },
                )
                .await
                .expect("a single question decision must be accepted")
                .answers,
        );
    }

    assert_eq!(joint_answers, separate_answers);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
