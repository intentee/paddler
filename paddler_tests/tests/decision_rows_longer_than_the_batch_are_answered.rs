#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const SMALL_BATCH_TOKENS: u32 = 16;
const PROBABILITY_SUM_TOLERANCE: f32 = 1e-5;

fn long_question(id: &str) -> DecisionQuestion {
    DecisionQuestion {
        id: id.to_owned(),
        instructions: "Read the whole ticket carefully and decide how the customer feels about \
                       the delivery, the payment and the support they received."
            .to_owned(),
        options: vec![
            "positive, because everything went well".to_owned(),
            "negative, because the delivery was slow and the payment was late".to_owned(),
        ],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn decision_rows_longer_than_the_batch_are_answered() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        n_batch: BatchSize::try_from(SMALL_BATCH_TOKENS).expect("the batch size is valid"),
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &RawDecideParams {
                questions: vec![long_question("first"), long_question("second")],
                state: sample_decision().state,
            },
        )
        .await
        .expect("the decision must be accepted");

    assert_eq!(
        collected
            .answers
            .iter()
            .map(|answer| answer.id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert!(collected.answers.iter().all(|answer| {
        (answer.probabilities.iter().sum::<f32>() - 1.0).abs() < PROBABILITY_SUM_TOLERANCE
    }));
    assert!(matches!(collected.terminal_result, DecisionResult::Done(_)));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
