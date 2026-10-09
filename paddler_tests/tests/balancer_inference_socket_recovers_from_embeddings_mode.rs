#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::next_reported_cluster_inference_mode::next_reported_cluster_inference_mode;
use paddler_tests::start_cluster::start_cluster;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();

fn capital_of_france_prompt() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: MAX_TOKENS,
        raw_prompt: "The capital of France is".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_socket_recovers_from_embeddings_mode() {
    let generation_state = qwen3_0_6b().into_desired_state();
    let embeddings_state = BalancerDesiredState {
        inference_mode: InferenceMode::Embeddings,
        ..generation_state.clone()
    };

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(embeddings_state)),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster serving embeddings must start");

    let mut cluster_inference_mode_rx = cluster
        .client_inference
        .subscribe_to_cluster_inference_mode();

    let refused_message = cluster
        .client_inference
        .continue_from_raw_prompt(CancellationToken::new(), capital_of_france_prompt())
        .await
        .expect("the inference socket must accept a request while serving embeddings")
        .next()
        .await
        .expect("the inference socket must answer while serving embeddings")
        .expect("the answer must be readable");

    assert!(matches!(
        refused_message,
        InferenceMessage::Response(envelope)
            if matches!(
                &envelope.response,
                InferenceResponse::GeneratedToken(GeneratedTokenResult::InferenceModeMismatch(description))
                    if description == "The cluster serves Embeddings, not TextGeneration"
            )
    ));

    let connect_notification = next_reported_cluster_inference_mode(&mut cluster_inference_mode_rx)
        .await
        .expect("the client must be told on connect that the cluster serves embeddings");

    assert_eq!(
        connect_notification,
        ReportedClusterInferenceMode::Reported(InferenceMode::Embeddings)
    );

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &generation_state)
        .await
        .expect("the balancer must accept the text generation state");

    let recovery_notification = next_reported_cluster_inference_mode(
        &mut cluster_inference_mode_rx,
    )
    .await
    .expect(
        "the client must be told over the open connection that the cluster serves text generation",
    );

    assert_eq!(
        recovery_notification,
        ReportedClusterInferenceMode::Reported(InferenceMode::TextGeneration)
    );

    cluster
        .wait_for_first_agent_to_serve(InferenceMode::TextGeneration)
        .await
        .expect("the agent must reload into text generation");

    let recovered = collect_generated_tokens(
        cluster
            .client_inference
            .continue_from_raw_prompt(CancellationToken::new(), capital_of_france_prompt())
            .await
            .expect("the recovered inference socket must accept a request"),
    )
    .await
    .expect("the recovered inference socket must stream tokens");

    assert!(!recovered.text.is_empty());

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
