#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_decisions_also_serves_completions() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("a decision cluster must start");

    let decided = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision must be accepted");
    let generated = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the raw prompt must be accepted");

    assert_eq!(
        discriminant(&decided.terminal_result),
        decision_done_discriminant()
    );
    assert_eq!(
        generated
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
