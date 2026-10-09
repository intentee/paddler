#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agents_serve_the_inference_mode_the_cluster_switches_to_at_runtime() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("a decision cluster must start");

    let decided = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision cluster must answer a decision");

    assert!(matches!(decided.terminal_result, DecisionResult::Done(_)));

    let decision_desired_state = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the balancer must report its desired state");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                inference_mode: InferenceMode::TextGeneration,
                ..decision_desired_state
            },
        )
        .await
        .expect("the balancer must switch the cluster to text generation");
    cluster
        .wait_for_first_agent_to_serve(InferenceMode::TextGeneration)
        .await
        .expect("the agent must reload into text generation");

    let generated = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: MAX_TOKENS,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the switched cluster must generate tokens");

    assert!(!generated.text.is_empty());
    generated
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
