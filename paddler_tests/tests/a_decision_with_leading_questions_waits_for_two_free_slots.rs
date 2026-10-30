#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

use crate::slot_held_by_an_unending_generation::SlotHeldByAnUnendingGeneration;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_with_leading_questions_waits_for_two_free_slots() {
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

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
