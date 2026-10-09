#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const SMALL_CONTEXT_SIZE: u32 = 256;
const STATE_REPETITIONS_EXCEEDING_THE_CONTEXT: usize = 20;

#[tokio::test(flavor = "multi_thread")]
async fn decision_exceeding_the_context_is_rejected() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        context_size: NonZeroU32::new(SMALL_CONTEXT_SIZE).expect("the context size is non-zero"),
        n_batch: BatchSize::try_from(SMALL_CONTEXT_SIZE).expect("the batch fits the context"),
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &RawDecideParams {
                state: sample_decision()
                    .state
                    .repeat(STATE_REPETITIONS_EXCEEDING_THE_CONTEXT),
                ..sample_decision()
            },
        )
        .await
        .expect("the oversized decision must be answered with a rejection");

    assert!(collected.answers.is_empty());
    assert!(matches!(
        collected.terminal_result,
        DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
            context_size,
            required_tokens,
        }) if context_size >= SMALL_CONTEXT_SIZE && required_tokens > context_size as usize
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
