#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const SMALL_BATCH_TOKENS: u32 = 32;
const STATE_REPETITIONS_SPANNING_MANY_DECODES: usize = 120;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_decision_client_releases_its_sequences() {
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        agents: vec![AgentConfig::single(2)],
        n_batch: BatchSize::try_from(SMALL_BATCH_TOKENS).expect("the batch size is valid"),
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have an agent")
        .clone();
    let mut half_closed_client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.inference,
        ApiPath::DECIDE,
        &RawDecideParams {
            state: sample_decision()
                .state
                .repeat(STATE_REPETITIONS_SPANNING_MANY_DECODES),
            ..sample_decision()
        },
    )
    .await
    .expect("the half-closed decision must be sent");

    half_closed_client
        .half_close()
        .await
        .expect("the decision must be half-closed");
    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the agent must release the abandoned decision");
    drop(half_closed_client);

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
