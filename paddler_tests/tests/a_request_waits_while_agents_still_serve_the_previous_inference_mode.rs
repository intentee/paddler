#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn a_request_waits_while_agents_still_serve_the_previous_inference_mode() {
    let mut cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("a decision cluster must start");

    let decision_desired_state = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the balancer must report its desired state");
    let text_generation_desired_state = BalancerDesiredState {
        inference_mode: InferenceMode::TextGeneration,
        ..decision_desired_state
    };

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
                ..text_generation_desired_state.clone()
            },
        )
        .await
        .expect("the balancer must switch the cluster to text generation");

    cluster
        .wait_for_first_agent_issue(|issue| matches!(issue, AgentIssue::ModelFileDoesNotExist(_)))
        .await
        .expect("the agent must keep serving decisions while the desired model is missing");

    let generation = spawn(cluster.continue_from_raw_prompt(
        CancellationToken::new(),
        &ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: MAX_TOKENS,
            raw_prompt: "The capital of France is".to_owned(),
        },
    ));

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the request must wait in the buffer");

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &text_generation_desired_state)
        .await
        .expect("the balancer must accept a text generation model the agent can load");

    generation
        .await
        .expect("the generation task must not panic")
        .expect("the buffered request must be answered")
        .summary()
        .expect("the buffered request must be served by an agent serving text generation");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
