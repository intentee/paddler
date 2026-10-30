#![cfg(feature = "tests_that_use_llms")]

use futures_util::future::join_all;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const CONCURRENT_DECISIONS: usize = 5;

#[tokio::test(flavor = "multi_thread")]
async fn decisions_are_identical_alone_and_concurrently() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        agents: vec![AgentConfig::single(2)],
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let decisions: Vec<RawDecideParams> = (0..CONCURRENT_DECISIONS)
        .map(|decision_index| RawDecideParams {
            state: format!("Ticket {decision_index}: {}", sample_decision().state),
            ..sample_decision()
        })
        .collect();
    let mut answers_alone = Vec::new();

    for decision in &decisions {
        answers_alone.push(
            cluster
                .decide(CancellationToken::new(), decision)
                .await
                .expect("a decision alone must be accepted")
                .answers,
        );
    }

    let answers_concurrently: Vec<_> = join_all(
        decisions
            .iter()
            .map(|decision| cluster.decide(CancellationToken::new(), decision)),
    )
    .await
    .into_iter()
    .map(|collected| {
        collected
            .expect("a concurrent decision must be accepted")
            .answers
    })
    .collect();

    assert_eq!(answers_concurrently, answers_alone);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
