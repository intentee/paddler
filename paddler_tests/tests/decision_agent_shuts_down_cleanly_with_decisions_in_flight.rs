#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::many_question_decision::many_question_decision;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const QUESTIONS_OUTLASTING_A_SHUTDOWN: usize = 96;

#[tokio::test(flavor = "multi_thread")]
async fn decision_agent_shuts_down_cleanly_with_decisions_in_flight() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have an agent")
        .clone();
    let mut answering_stream = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &many_question_decision(QUESTIONS_OUTLASTING_A_SHUTDOWN),
        )
        .await
        .expect("the long decision must be accepted");

    answering_stream
        .next()
        .await
        .expect("the long decision must answer its first question")
        .expect("the first answer must be readable");

    let waiting_stream = cluster
        .client_inference
        .post_decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the waiting decision must be accepted");

    cluster
        .wait_for_slots_processing(&agent_id, 2)
        .await
        .expect("both decisions must reach the agent");
    answering_stream
        .next()
        .await
        .expect("the long decision must keep answering")
        .expect("the next answer must be readable");

    cluster
        .agents
        .pop()
        .expect("the cluster must have an agent")
        .shutdown()
        .await
        .expect("the agent must shut down cleanly with decisions in flight");

    drop(answering_stream);
    drop(waiting_stream);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
