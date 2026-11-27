#![cfg(feature = "tests_that_use_llms")]

use futures_util::future::join_all;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_request_params_fixture::ticket_decision::ticket_decision;

const CONCURRENT_DECISIONS: usize = 5;

#[tokio::test(flavor = "multi_thread")]
async fn decisions_are_identical_alone_and_concurrently() {
    let cluster = DecisionClusterParams {
        agents: vec![AgentConfig::single(2)],
        ..DecisionClusterParams::default()
    }
    .start_cluster()
    .await
    .expect("the cluster must start");
    let decisions: Vec<DecisionRequest> = (0..CONCURRENT_DECISIONS).map(ticket_decision).collect();
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
