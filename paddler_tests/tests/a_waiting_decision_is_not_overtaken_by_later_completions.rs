#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

use crate::slot_held_by_an_unending_generation::SlotHeldByAnUnendingGeneration;

#[tokio::test(flavor = "multi_thread")]
async fn a_waiting_decision_is_not_overtaken_by_later_completions() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("a decision cluster must start");
    let slot_held_by_an_unending_generation =
        SlotHeldByAnUnendingGeneration::hold(&mut cluster).await;
    let decision = spawn(cluster.decide(CancellationToken::new(), &sample_decision()));

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the decision must wait while only one slot is free");

    let completion = spawn(cluster.continue_from_raw_prompt(
        CancellationToken::new(),
        &ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            raw_prompt: "The capital of France is".to_owned(),
        },
    ));

    cluster
        .wait_for_buffered_request_count(2)
        .await
        .expect("the completion must wait behind the decision although a slot is free");
    slot_held_by_an_unending_generation.release().await;

    assert_eq!(
        discriminant(
            &decision
                .await
                .expect("the decision task must not panic")
                .expect("the decision must be answered")
                .terminal_result
        ),
        decision_done_discriminant()
    );
    assert_eq!(
        completion
            .await
            .expect("the completion task must not panic")
            .expect("the completion must be answered")
            .summary()
            .expect("the completion must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
