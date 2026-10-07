#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::many_question_decision::many_question_decision;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const QUESTIONS_OUTLASTING_A_DISCONNECT: usize = 96;

#[tokio::test(flavor = "multi_thread")]
async fn decision_client_disconnecting_mid_decision_releases_its_sequences() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have an agent")
        .clone();
    let mut decision_stream = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &many_question_decision(QUESTIONS_OUTLASTING_A_DISCONNECT),
        )
        .await
        .expect("the decision must be accepted");

    decision_stream
        .next()
        .await
        .expect("the decision must answer its first question")
        .expect("the first answer must be readable");
    drop(decision_stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the agent must release the abandoned decision");

    let collected = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("a decision after the abandoned one must be accepted");

    assert_eq!(collected.answers.len(), sample_decision().questions.len());
    assert!(matches!(collected.terminal_result, DecisionResult::Done(_)));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
