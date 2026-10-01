#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
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
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..generation_state.inference_parameters.clone()
        },
        ..generation_state.clone()
    };

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: Some(embeddings_state),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster serving embeddings must start");

    let inference = &cluster.client_inference;
    let mut token_generation_mode_rx = inference.subscribe_to_token_generation_mode();

    let mut disabled_stream = inference
        .continue_from_raw_prompt(CancellationToken::new(), capital_of_france_prompt())
        .await
        .expect("the inference socket must accept a request in embeddings mode");

    let disabled_message = disabled_stream
        .next()
        .await
        .expect("inference socket must answer instead of rejecting in embeddings mode")
        .expect("the embeddings-mode answer must be readable");

    match disabled_message {
        InferenceMessage::Response(envelope) => match envelope.response {
            InferenceResponse::GeneratedToken(GeneratedTokenResult::TokenGenerationDisabled(_)) => {
            }
            other => panic!("expected a token-generation-disabled reply, got {other:?}"),
        },
        other => panic!("expected a token-generation-disabled reply, got {other:?}"),
    }

    let connect_notification = token_generation_mode_rx
        .recv()
        .await
        .expect("client must be told on connect that token generation is disabled");

    assert!(matches!(
        connect_notification,
        Notification::TokenGenerationDisabled
    ));

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &generation_state)
        .await
        .expect("the balancer must accept the token generation state");

    let recovery_notification = token_generation_mode_rx.recv().await.expect(
        "client must be told over the open connection that token generation is enabled again",
    );

    assert!(matches!(
        recovery_notification,
        Notification::TokenGenerationEnabled
    ));

    let mut recovered_stream = inference
        .continue_from_raw_prompt(CancellationToken::new(), capital_of_france_prompt())
        .await
        .expect("the recovered inference socket must accept a request");

    let mut generated_token_count: usize = 0;

    while let Some(message_result) = recovered_stream.next().await {
        match message_result.expect("the recovered stream must stay readable") {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::GeneratedToken(token_result) => {
                    if token_result.is_token() {
                        generated_token_count += 1;
                    }
                }
                other @ InferenceResponse::Embedding(_) => {
                    panic!("unexpected response after recovery: {other:?}")
                }
            },
            InferenceMessage::Error(envelope) => panic!(
                "recovered inference failed: code {}, description {:?}",
                envelope.error.code, envelope.error.description
            ),
            InferenceMessage::Notification(_) => {}
        }
    }

    assert!(
        generated_token_count > 0,
        "the recovered connection must stream tokens once token generation is enabled again"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
