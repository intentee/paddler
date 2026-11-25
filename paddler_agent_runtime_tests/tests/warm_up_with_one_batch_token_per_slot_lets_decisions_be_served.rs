#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_cluster_harness::decision_done_discriminant::decision_done_discriminant;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_request_params_fixture::sample_decision::sample_decision;

const LONGEST_SAMPLE_DECISION_READ_SPAN: u16 = 12;
const SLOTS: u16 = LONGEST_SAMPLE_DECISION_READ_SPAN;

#[tokio::test(flavor = "multi_thread")]
async fn warm_up_with_one_batch_token_per_slot_lets_decisions_be_served() {
    let cluster = DecisionClusterParams {
        agents: vec![AgentConfig::single(SLOTS)],
        desired_state: DecisionDesiredState {
            n_batch: BatchSize::try_from(u32::from(SLOTS))
                .expect("a batch with one token per slot is valid"),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
    .start_cluster()
    .await
    .expect("the cluster must start");

    let collected = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision must be accepted");

    assert_eq!(collected.answers.len(), sample_decision().questions.len());
    assert_eq!(
        discriminant(&collected.terminal_result),
        decision_done_discriminant()
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
