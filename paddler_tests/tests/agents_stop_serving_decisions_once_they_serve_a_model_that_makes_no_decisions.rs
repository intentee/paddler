#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_tests::start_decision_cluster_that_answered_a_decision::start_decision_cluster_that_answered_a_decision;

#[tokio::test(flavor = "multi_thread")]
async fn agents_stop_serving_decisions_once_they_serve_a_model_that_makes_no_decisions() {
    let mut cluster = start_decision_cluster_that_answered_a_decision()
        .await
        .expect("a decision cluster must start and answer a decision");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &qwen3_5_0_8b().into_desired_state(),
        )
        .await
        .expect("the balancer must accept a model that makes no decisions");
    cluster
        .wait_for_first_agent_to_serve(AgentDecisionsServing::NotServed)
        .await
        .expect("the agent must stop serving decisions");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
