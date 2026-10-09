#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::decision_result::DecisionResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_with_a_batch_too_small_to_warm_up_every_slot() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        agents: vec![AgentConfig::single(SLOTS)],
        n_batch: BatchSize::try_from(u32::from(SLOTS))
            .expect("a batch with one token per slot is valid"),
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision must be accepted");

    assert_eq!(collected.answers.len(), sample_decision().questions.len());
    assert!(matches!(collected.terminal_result, DecisionResult::Done(_)));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
