#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_done_discriminant::decision_done_discriminant;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agents_stop_serving_decisions_once_the_pointer_head_is_removed() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("a decision cluster must start");

    let decided = cluster
        .decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("the decision cluster must answer a decision");

    assert_eq!(
        discriminant(&decided.terminal_result),
        decision_done_discriminant()
    );

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
                decision: DecisionSettings::default(),
                ..decision_desired_state
            },
        )
        .await
        .expect("the balancer must stop serving decisions");
    cluster
        .wait_for_first_agent_to_serve(AgentDecisionsServing::NotServed)
        .await
        .expect("the agent must stop serving decisions");

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
        .expect("the cluster without a pointer head must generate tokens");

    assert_ne!(generated.text, "");
    generated
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
