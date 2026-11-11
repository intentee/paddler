#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::start_decision_cluster_that_answered_a_decision::start_decision_cluster_that_answered_a_decision;

#[tokio::test(flavor = "multi_thread")]
async fn agents_stop_serving_decisions_once_the_pointer_head_is_removed() {
    let mut cluster = start_decision_cluster_that_answered_a_decision()
        .await
        .expect("a decision cluster must start and answer a decision");

    cluster
        .remove_pointer_head()
        .await
        .expect("the agent must stop serving decisions once the pointer head is removed");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
