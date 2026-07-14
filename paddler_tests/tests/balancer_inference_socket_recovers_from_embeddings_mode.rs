#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

// Failure ceiling for the agent to reload out of embeddings mode into token
// generation: teardown of the embeddings context, reload of the 0.6B weights
// from the local cache, context re-init, and chat-template load complete in a
// few seconds; this is generous headroom so the buffered request waits out the
// reload while still failing in bounded time if recovery never happens.
const MODEL_RELOAD_CEILING: Duration = Duration::from_mins(2);

fn capital_of_france_prompt() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 16,
        raw_prompt: "The capital of France is".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_socket_recovers_from_embeddings_mode() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let embeddings_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        },
        model: AgentDesiredModel::HuggingFace(reference.clone()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        buffered_request_timeout: MODEL_RELOAD_CEILING,
        desired_state: Some(embeddings_state),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    let inference = &cluster.client_inference;
    let mut token_generation_mode_rx = inference.subscribe_to_token_generation_mode();

    let mut disabled_stream = inference
        .continue_from_raw_prompt(capital_of_france_prompt())
        .await
        .map_err(anyhow::Error::new)?;

    let disabled_message = disabled_stream
        .next()
        .await
        .context("inference socket must answer instead of rejecting in embeddings mode")?
        .map_err(anyhow::Error::new)?;

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
        .context("client must be told on connect that token generation is disabled")?;

    assert!(matches!(
        connect_notification,
        Notification::TokenGenerationDisabled
    ));

    let generation_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            enable_embeddings: false,
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    cluster
        .client_management
        .put_balancer_desired_state(&generation_state)
        .await
        .map_err(anyhow::Error::new)?;

    let recovery_notification = token_generation_mode_rx.recv().await.context(
        "client must be told over the open connection that token generation is enabled again",
    )?;

    assert!(matches!(
        recovery_notification,
        Notification::TokenGenerationEnabled
    ));

    let mut recovered_stream = inference
        .continue_from_raw_prompt(capital_of_france_prompt())
        .await
        .map_err(anyhow::Error::new)?;

    let mut generated_token_count: usize = 0;

    while let Some(message_result) = recovered_stream.next().await {
        match message_result.map_err(anyhow::Error::new)? {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::GeneratedToken(token_result) => {
                    if token_result.is_token() {
                        generated_token_count += 1;
                    }
                }
                other => panic!("unexpected response after recovery: {other:?}"),
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

    cluster.shutdown().await?;

    Ok(())
}
