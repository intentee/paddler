#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::laya::laya;

const BATCH_SHORTER_THAN_THE_PROMPT: u32 = 16;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_prompt_a_model_without_causal_attention_cannot_decode_in_one_batch_is_refused()
{
    let cluster = DecisionClusterParams {
        desired_state: DecisionDesiredState {
            model_card: laya(),
            n_batch: BatchSize::try_from(BATCH_SHORTER_THAN_THE_PROMPT)
                .expect("the batch size is valid"),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
    .start_cluster()
    .await
    .expect("the cluster must start");

    let collected = cluster
        .decide(CancellationToken::new(), &snowy_weather_decision())
        .await
        .expect("the decision must be answered with a refusal");

    assert_eq!(collected.answers, Vec::new());
    assert!(matches!(
        collected.terminal_result,
        DecisionResult::RequestRejected(_)
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
